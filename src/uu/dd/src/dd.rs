// This file is part of the uutils coreutils package.
//
// (c) Tyler Steele <tyler.steele@protonmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[macro_use]
extern crate uucore;
use uucore::InvalidEncodingHandling;

#[cfg(test)]
mod dd_unit_tests;

mod datastructures;
use datastructures::*;

mod parseargs;
use parseargs::Matches;

mod conversion_tables;
use conversion_tables::*;

use byte_unit::Byte;
use clap::{self, crate_version};
use debug_print::debug_println;
use gcd::Gcd;
use signal_hook::consts::signal;
use std::cmp;
use std::convert::TryInto;
use std::env;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, Write};
#[cfg(target_os = "linux")]
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::{atomic::AtomicUsize, atomic::Ordering, mpsc, Arc};
use std::thread;
use std::time;

// const SYNTAX: &str = "dd [OPERAND]...\ndd OPTION";
const ABOUT: &str = "copy, and optionally convert, a file system resource";
const BUF_INIT_BYTE: u8 = 0xDD;
const RTN_SUCCESS: i32 = 0;
const RTN_FAILURE: i32 = 1;

struct Input<R: Read> {
    src: R,
    non_ascii: bool,
    ibs: usize,
    xfer_stats: Option<StatusLevel>,
    count: Option<CountType>,
    cflags: IConvFlags,
    iflags: IFlags,
}

impl Input<io::Stdin> {
    fn new(matches: &Matches) -> Result<Self, Box<dyn Error>> {
        let ibs = parseargs::parse_ibs(matches)?;
        let non_ascii = parseargs::parse_input_non_ascii(matches)?;
        let xfer_stats = parseargs::parse_status_level(matches)?;
        let cflags = parseargs::parse_conv_flag_input(matches)?;
        let iflags = parseargs::parse_iflags(matches)?;
        let skip = parseargs::parse_skip_amt(&ibs, &iflags, matches)?;
        let count = parseargs::parse_count(&iflags, matches)?;

        let mut i = Input {
            src: io::stdin(),
            non_ascii,
            ibs,
            xfer_stats,
            count,
            cflags,
            iflags,
        };

        if let Some(amt) = skip {
            let mut buf = vec![BUF_INIT_BYTE; amt];

            i.force_fill(&mut buf, amt)?;
        }

        Ok(i)
    }
}

#[cfg(target_os = "linux")]
fn make_linux_iflags(oflags: &IFlags) -> Option<libc::c_int> {
    let mut flag = 0;

    if oflags.direct {
        flag |= libc::O_DIRECT;
    }
    if oflags.directory {
        flag |= libc::O_DIRECTORY;
    }
    if oflags.dsync {
        flag |= libc::O_DSYNC;
    }
    if oflags.noatime {
        flag |= libc::O_NOATIME;
    }
    if oflags.noctty {
        flag |= libc::O_NOCTTY;
    }
    if oflags.nofollow {
        flag |= libc::O_NOFOLLOW;
    }
    if oflags.nonblock {
        flag |= libc::O_NONBLOCK;
    }
    if oflags.sync {
        flag |= libc::O_SYNC;
    }

    if flag != 0 {
        Some(flag)
    } else {
        None
    }
}

impl Input<File> {
    fn new(matches: &Matches) -> Result<Self, Box<dyn Error>> {
        let ibs = parseargs::parse_ibs(matches)?;
        let non_ascii = parseargs::parse_input_non_ascii(matches)?;
        let xfer_stats = parseargs::parse_status_level(matches)?;
        let cflags = parseargs::parse_conv_flag_input(matches)?;
        let iflags = parseargs::parse_iflags(matches)?;
        let skip = parseargs::parse_skip_amt(&ibs, &iflags, matches)?;
        let count = parseargs::parse_count(&iflags, matches)?;

        if let Some(fname) = matches.value_of("if") {
            let mut src = {
                let mut opts = OpenOptions::new();
                opts.read(true);

                #[cfg(target_os = "linux")]
                if let Some(libc_flags) = make_linux_iflags(&iflags) {
                    opts.custom_flags(libc_flags);
                }

                opts.open(fname)?
            };

            if let Some(amt) = skip {
                let amt: u64 = amt.try_into()?;
                src.seek(io::SeekFrom::Start(amt))?;
            }

            let i = Input {
                src,
                non_ascii,
                ibs,
                xfer_stats,
                count,
                cflags,
                iflags,
            };

            Ok(i)
        } else {
            Err(Box::new(InternalError::WrongInputType))
        }
    }
}

impl<R: Read> Read for Input<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut base_idx = 0;
        let tlen = buf.len();
        loop {
            match self.src.read(&mut buf[base_idx..]) {
                Ok(0) => return Ok(base_idx),
                Ok(rlen) if self.iflags.fullblock => {
                    base_idx += rlen;

                    if base_idx >= tlen {
                        return Ok(tlen);
                    }
                }
                Ok(len) => return Ok(len),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) if self.cflags.noerror => return Ok(base_idx),
                Err(e) => return Err(e),
            }
        }
    }
}

impl<R: Read> Input<R> {
    /// Fills a given buffer.
    /// Reads in increments of 'self.ibs'.
    /// The start of each ibs-sized read follows the previous one.
    fn fill_consecutive(&mut self, buf: &mut Vec<u8>) -> Result<ReadStat, Box<dyn Error>> {
        let mut reads_complete = 0;
        let mut reads_partial = 0;
        let mut base_idx = 0;

        while base_idx < buf.len() {
            let next_blk = cmp::min(base_idx + self.ibs, buf.len());

            match self.read(&mut buf[base_idx..next_blk])? {
                rlen if rlen == self.ibs => {
                    base_idx += rlen;
                    reads_complete += 1;
                }
                rlen if rlen > 0 => {
                    base_idx += rlen;
                    reads_partial += 1;
                }
                _ => break,
            }
        }

        buf.truncate(base_idx);
        Ok(ReadStat {
            reads_complete,
            reads_partial,
            records_truncated: 0,
        })
    }

    /// Fills a given buffer.
    /// Reads in increments of 'self.ibs'.
    /// The start of each ibs-sized read is aligned to multiples of ibs; remaining space is filled with the 'pad' byte.
    fn fill_blocks(&mut self, buf: &mut Vec<u8>, pad: u8) -> Result<ReadStat, Box<dyn Error>> {
        let mut reads_complete = 0;
        let mut reads_partial = 0;
        let mut base_idx = 0;

        while base_idx < buf.len() {
            let next_blk = cmp::min(base_idx + self.ibs, buf.len());
            let plen = next_blk - base_idx;

            match self.read(&mut buf[base_idx..next_blk])? {
                0 => break,
                rlen if rlen < plen => {
                    reads_partial += 1;
                    let padding = vec![pad; plen - rlen];
                    buf.splice(base_idx + rlen..next_blk, padding.into_iter());
                }
                _ => {
                    reads_complete += 1;
                }
            }
            // TODO: Why does this cause the conv=sync tests to hang?
            // let rlen = self.read(&mut buf[base_idx..next_blk])?;
            // if rlen < plen
            // {
            //     reads_partial += 1;
            //     let padding = vec![pad; plen-rlen];
            //     buf.splice(base_idx+rlen..next_blk, padding.into_iter());
            // }
            // else
            // {
            //     reads_complete += 1;
            // }
            // if rlen == 0
            // {
            //     break;
            // }

            base_idx += self.ibs;
        }

        buf.truncate(base_idx);
        Ok(ReadStat {
            reads_complete,
            reads_partial,
            records_truncated: 0,
        })
    }

    /// Force-fills a buffer, ignoring zero-length reads which would otherwise be
    /// interpreted as EOF.
    /// Note: This will not return unless the source (eventually) produces
    /// enough bytes to meet target_len.
    fn force_fill(&mut self, buf: &mut [u8], target_len: usize) -> Result<usize, Box<dyn Error>> {
        let mut base_idx = 0;
        while base_idx < target_len {
            base_idx += self.read(&mut buf[base_idx..target_len])?;
        }

        Ok(base_idx)
    }
}

struct Output<W: Write> {
    dst: W,
    obs: usize,
    cflags: OConvFlags,
}

impl Output<io::Stdout> {
    fn new(matches: &Matches) -> Result<Self, Box<dyn Error>> {
        let obs = parseargs::parse_obs(matches)?;
        let cflags = parseargs::parse_conv_flag_output(matches)?;

        let dst = io::stdout();

        Ok(Output { dst, obs, cflags })
    }

    fn fsync(&mut self) -> io::Result<()> {
        self.dst.flush()
    }

    fn fdatasync(&mut self) -> io::Result<()> {
        self.dst.flush()
    }
}

#[cfg(target_os = "linux")]
fn make_linux_oflags(oflags: &OFlags) -> Option<libc::c_int> {
    let mut flag = 0;

    // oflag=FLAG
    if oflags.append {
        flag |= libc::O_APPEND;
    }
    if oflags.direct {
        flag |= libc::O_DIRECT;
    }
    if oflags.directory {
        flag |= libc::O_DIRECTORY;
    }
    if oflags.dsync {
        flag |= libc::O_DSYNC;
    }
    if oflags.noatime {
        flag |= libc::O_NOATIME;
    }
    if oflags.noctty {
        flag |= libc::O_NOCTTY;
    }
    if oflags.nofollow {
        flag |= libc::O_NOFOLLOW;
    }
    if oflags.nonblock {
        flag |= libc::O_NONBLOCK;
    }
    if oflags.sync {
        flag |= libc::O_SYNC;
    }

    if flag != 0 {
        Some(flag)
    } else {
        None
    }
}

impl Output<File> {
    fn new(matches: &Matches) -> Result<Self, Box<dyn Error>> {
        fn open_dst(
            path: &Path,
            cflags: &OConvFlags,
            oflags: &OFlags,
        ) -> Result<File, Box<dyn Error>> {
            let mut opts = OpenOptions::new();
            opts.write(true)
                .create(!cflags.nocreat)
                .truncate(!cflags.notrunc)
                .create_new(cflags.excl)
                .append(oflags.append);

            #[cfg(target_os = "linux")]
            if let Some(libc_flags) = make_linux_oflags(oflags) {
                opts.custom_flags(libc_flags);
            }

            let dst = opts.open(path)?;
            Ok(dst)
        }
        let obs = parseargs::parse_obs(matches)?;
        let cflags = parseargs::parse_conv_flag_output(matches)?;
        let oflags = parseargs::parse_oflags(matches)?;
        let seek = parseargs::parse_seek_amt(&obs, &oflags, matches)?;

        if let Some(fname) = matches.value_of("of") {
            let mut dst = open_dst(Path::new(&fname), &cflags, &oflags)?;

            if let Some(amt) = seek {
                let amt: u64 = amt.try_into()?;
                dst.seek(io::SeekFrom::Start(amt))?;
            }

            Ok(Output { dst, obs, cflags })
        } else {
            // The following error should only occur if someone
            // mistakenly calls Output::<File>::new() without checking
            // if 'of' has been provided. In this case,
            // Output::<io::stdout>::new() is probably intended.
            Err(Box::new(InternalError::WrongOutputType))
        }
    }

    fn fsync(&mut self) -> io::Result<()> {
        self.dst.flush()?;
        self.dst.sync_all()
    }

    fn fdatasync(&mut self) -> io::Result<()> {
        self.dst.flush()?;
        self.dst.sync_data()
    }
}

impl Seek for Output<File> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.dst.seek(pos)
    }
}

impl Write for Output<File> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        #[inline]
        fn is_sparse(buf: &[u8]) -> bool {
            buf.iter().all(|&e| e == 0u8)
        }
        // -----------------------------
        if self.cflags.sparse && is_sparse(buf) {
            let seek_amt: i64 = buf
                .len()
                .try_into()
                .expect("Internal dd Error: Seek amount greater than signed 64-bit integer");
            self.dst.seek(io::SeekFrom::Current(seek_amt))?;
            Ok(buf.len())
        } else {
            self.dst.write(buf)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.dst.flush()
    }
}

impl Write for Output<io::Stdout> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.dst.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.dst.flush()
    }
}

impl Output<io::Stdout> {
    fn write_blocks(&mut self, buf: Vec<u8>) -> io::Result<WriteStat> {
        let mut writes_complete = 0;
        let mut writes_partial = 0;
        let mut base_idx = 0;

        while base_idx < buf.len() {
            let next_blk = cmp::min(base_idx + self.obs, buf.len());
            let plen = next_blk - base_idx;

            match self.write(&buf[base_idx..next_blk])? {
                wlen if wlen < plen => {
                    writes_partial += 1;
                    base_idx += wlen;
                }
                wlen => {
                    writes_complete += 1;
                    base_idx += wlen;
                }
            }
        }

        Ok(WriteStat {
            writes_complete,
            writes_partial,
            bytes_total: base_idx.try_into().unwrap_or(0u128),
        })
    }
}

impl Output<File> {
    fn write_blocks(&mut self, buf: Vec<u8>) -> io::Result<WriteStat> {
        let mut writes_complete = 0;
        let mut writes_partial = 0;
        let mut base_idx = 0;

        while base_idx < buf.len() {
            let next_blk = cmp::min(base_idx + self.obs, buf.len());
            let plen = next_blk - base_idx;

            match self.write(&buf[base_idx..next_blk])? {
                wlen if wlen < plen => {
                    writes_partial += 1;
                    base_idx += wlen;
                }
                wlen => {
                    writes_complete += 1;
                    base_idx += wlen;
                }
            }
        }

        Ok(WriteStat {
            writes_complete,
            writes_partial,
            bytes_total: base_idx.try_into().unwrap_or(0u128),
        })
    }
}

/// Splits the content of buf into cbs-length blocks
/// Appends padding as specified by conv=block and cbs=N
fn block(buf: Vec<u8>, cbs: usize, rstat: &mut ReadStat) -> Vec<Vec<u8>> {
    let mut blocks = buf
        .split(|&e| e == b'\n')
        .fold(Vec::new(), |mut blocks, split| {
            let mut split = split.to_vec();
            if split.len() > cbs {
                rstat.records_truncated += 1;
            }
            split.resize(cbs, b' ');
            blocks.push(split);

            blocks
        });

    if let Some(last) = blocks.last() {
        if last.iter().all(|&e| e == b' ') {
            blocks.pop();
        }
    }

    blocks
}

/// Trims padding from each cbs-length partition of buf
/// as specified by conv=unblock and cbs=N
fn unblock(buf: Vec<u8>, cbs: usize) -> Vec<u8> {
    // Local Helper Fns ----------------------------------------------------
    #[inline]
    fn build_blocks(buf: Vec<u8>, cbs: usize) -> Vec<Vec<u8>> {
        let mut blocks = Vec::new();
        let mut curr = buf;
        let mut next;
        let mut width;

        while !curr.is_empty() {
            width = cmp::min(cbs, curr.len());
            next = curr.split_off(width);

            blocks.push(curr);

            curr = next;
        }

        blocks
    }
    // ---------------------------------------------------------------------
    build_blocks(buf, cbs)
        .into_iter()
        .fold(Vec::new(), |mut unblocks, mut block| {
            let block = if let Some(last_char_idx) = block.iter().rposition(|&e| e != b' ') {
                block.truncate(last_char_idx + 1);
                block.push(b'\n');

                block
            } else if let Some(b' ') = block.get(0) {
                vec![b'\n']
            } else {
                block
            };

            unblocks.push(block);

            unblocks
        })
        .into_iter()
        .flatten()
        .collect()
}

fn conv_block_unblock_helper<R: Read>(
    mut buf: Vec<u8>,
    i: &mut Input<R>,
    rstat: &mut ReadStat,
) -> Result<Vec<u8>, Box<dyn Error>> {
    // Local Predicate Fns -------------------------------------------------
    #[inline]
    fn should_block_then_conv<R: Read>(i: &Input<R>) -> bool {
        !i.non_ascii && i.cflags.block.is_some()
    }
    #[inline]
    fn should_conv_then_block<R: Read>(i: &Input<R>) -> bool {
        i.non_ascii && i.cflags.block.is_some()
    }
    #[inline]
    fn should_unblock_then_conv<R: Read>(i: &Input<R>) -> bool {
        !i.non_ascii && i.cflags.unblock.is_some()
    }
    #[inline]
    fn should_conv_then_unblock<R: Read>(i: &Input<R>) -> bool {
        i.non_ascii && i.cflags.unblock.is_some()
    }
    fn conv_only<R: Read>(i: &Input<R>) -> bool {
        i.cflags.ctable.is_some() && i.cflags.block.is_none() && i.cflags.unblock.is_none()
    }
    // Local Helper Fns ----------------------------------------------------
    #[inline]
    fn apply_ct(buf: &mut [u8], ct: &ConversionTable) {
        for idx in 0..buf.len() {
            buf[idx] = ct[buf[idx] as usize];
        }
    }
    // --------------------------------------------------------------------
    if conv_only(i) {
        // no block/unblock
        let ct = i.cflags.ctable.unwrap();
        apply_ct(&mut buf, ct);

        Ok(buf)
    } else if should_block_then_conv(i) {
        // ascii input so perform the block first
        let cbs = i.cflags.block.unwrap();

        let mut blocks = block(buf, cbs, rstat);

        if let Some(ct) = i.cflags.ctable {
            for buf in blocks.iter_mut() {
                apply_ct(buf, ct);
            }
        }

        let blocks = blocks.into_iter().flatten().collect();

        Ok(blocks)
    } else if should_conv_then_block(i) {
        // Non-ascii so perform the conversion first
        let cbs = i.cflags.block.unwrap();

        if let Some(ct) = i.cflags.ctable {
            apply_ct(&mut buf, ct);
        }

        let blocks = block(buf, cbs, rstat).into_iter().flatten().collect();

        Ok(blocks)
    } else if should_unblock_then_conv(i) {
        // ascii input so perform the unblock first
        let cbs = i.cflags.unblock.unwrap();

        let mut buf = unblock(buf, cbs);

        if let Some(ct) = i.cflags.ctable {
            apply_ct(&mut buf, ct);
        }

        Ok(buf)
    } else if should_conv_then_unblock(i) {
        // Non-ascii input so perform the conversion first
        let cbs = i.cflags.unblock.unwrap();

        if let Some(ct) = i.cflags.ctable {
            apply_ct(&mut buf, ct);
        }

        let buf = unblock(buf, cbs);

        Ok(buf)
    } else {
        // The following error should not happen, as it results from
        // insufficient command line data. This case should be caught
        // by the parser before making it this far.
        // Producing this error is an alternative to risking an unwrap call
        // on 'cbs' if the required data is not provided.
        Err(Box::new(InternalError::InvalidConvBlockUnblockCase))
    }
}

fn read_helper<R: Read>(
    i: &mut Input<R>,
    bsize: usize,
) -> Result<(ReadStat, Vec<u8>), Box<dyn Error>> {
    // Local Predicate Fns -----------------------------------------------
    #[inline]
    fn is_conv<R: Read>(i: &Input<R>) -> bool {
        i.cflags.ctable.is_some()
    }
    #[inline]
    fn is_block<R: Read>(i: &Input<R>) -> bool {
        i.cflags.block.is_some()
    }
    #[inline]
    fn is_unblock<R: Read>(i: &Input<R>) -> bool {
        i.cflags.unblock.is_some()
    }
    // Local Helper Fns -------------------------------------------------
    #[inline]
    fn perform_swab(buf: &mut [u8]) {
        let mut tmp;

        for base in (1..buf.len()).step_by(2) {
            tmp = buf[base];
            buf[base] = buf[base - 1];
            buf[base - 1] = tmp;
        }
    }
    // ------------------------------------------------------------------
    // Read
    let mut buf = vec![BUF_INIT_BYTE; bsize];
    let mut rstat = match i.cflags.sync {
        Some(ch) => i.fill_blocks(&mut buf, ch)?,
        _ => i.fill_consecutive(&mut buf)?,
    };
    // Return early if no data
    if rstat.reads_complete == 0 && rstat.reads_partial == 0 {
        return Ok((rstat, buf));
    }

    // Perform any conv=x[,x...] options
    if i.cflags.swab {
        perform_swab(&mut buf);
    }
    if is_conv(i) || is_block(i) || is_unblock(i) {
        let buf = conv_block_unblock_helper(buf, i, &mut rstat)?;
        Ok((rstat, buf))
    } else {
        Ok((rstat, buf))
    }
}

fn print_io_lines(update: &ProgUpdate) {
    eprintln!(
        "{}+{} records in",
        update.reads_complete, update.reads_partial
    );
    if update.records_truncated > 0 {
        eprintln!("{} truncated records", update.records_truncated);
    }
    eprintln!(
        "{}+{} records out",
        update.writes_complete, update.writes_partial
    );
}
fn make_prog_line(update: &ProgUpdate) -> String {
    let btotal_metric = Byte::from_bytes(update.bytes_total)
        .get_appropriate_unit(false)
        .format(0);
    let btotal_bin = Byte::from_bytes(update.bytes_total)
        .get_appropriate_unit(true)
        .format(0);
    let safe_millis = cmp::max(1, update.duration.as_millis());
    let xfer_rate = Byte::from_bytes(1000 * (update.bytes_total / safe_millis))
        .get_appropriate_unit(false)
        .format(1);

    format!(
        "{} bytes ({}, {}) copied, {:.1} s, {}/s",
        update.bytes_total,
        btotal_metric,
        btotal_bin,
        update.duration.as_secs_f64(),
        xfer_rate
    )
}
fn reprint_prog_line(update: &ProgUpdate) {
    eprint!("\r{}", make_prog_line(update));
}
fn print_prog_line(update: &ProgUpdate) {
    eprintln!("{}", make_prog_line(update));
}
fn print_xfer_stats(update: &ProgUpdate) {
    print_io_lines(update);
    print_prog_line(update);
}

/// Generate a progress updater that tracks progress, receives updates, and responds to signals.
fn gen_prog_updater(rx: mpsc::Receiver<ProgUpdate>, xfer_stats: Option<StatusLevel>) -> impl Fn() {
    // --------------------------------------------------------------
    fn posixly_correct() -> bool {
        env::var("POSIXLY_CORRECT").is_ok()
    }
    // --------------------------------------------------------------
    move || {
        const SIGUSR1_USIZE: usize = signal::SIGUSR1 as usize;

        let sigval = Arc::new(AtomicUsize::new(0));

        // TODO: SIGINFO seems to only exist for BSD (and therefore MACOS)
        // I will probably want put this behind a feature-gate and may need to pass the value to handle as my own constant.
        // This may involve some finagling with the signals library.
        // see -> https://unix.stackexchange.com/questions/179481/siginfo-on-gnu-linux-arch-linux-missing
        // if let Err(e) = signal_hook::flag::register_usize(signal::SIGINFO, sigval.clone(), signal::SIGINFO as usize)
        // {
        //     debug_println!("Internal dd Warning: Unable to register SIGINFO handler \n\t{}", e);
        // }
        if !posixly_correct() {
            if let Err(e) =
                signal_hook::flag::register_usize(signal::SIGUSR1, sigval.clone(), SIGUSR1_USIZE)
            {
                debug_println!(
                    "Internal dd Warning: Unable to register SIGUSR1 handler \n\t{}",
                    e
                );
            }
        }

        loop {
            // Wait for update
            let update = match (rx.recv(), xfer_stats) {
                (Ok(update), Some(StatusLevel::Progress)) => {
                    reprint_prog_line(&update);

                    update
                }
                (Ok(update), _) => update,
                (Err(_), _) =>
                // recv only fails permanently
                {
                    break
                }
            };
            // Handle signals
            #[allow(clippy::single_match)]
            match sigval.load(Ordering::Relaxed) {
                SIGUSR1_USIZE => {
                    print_xfer_stats(&update);
                }
                // SIGINFO_USIZE => ...
                _ => { /* no signals recv'd */ }
            };
        }
    }
}

/// Calculate a 'good' internal buffer size.
/// For performance of the read/write functions, the buffer should hold
/// both an integral number of reads and an integral number of writes. For
/// sane real-world memory use, it should not be too large. I believe
/// the least common multiple is a good representation of these interests.
/// https://en.wikipedia.org/wiki/Least_common_multiple#Using_the_greatest_common_divisor
#[inline]
fn calc_bsize(ibs: usize, obs: usize) -> usize {
    let gcd = Gcd::gcd(ibs, obs);
    // calculate the lcm from gcd
    (ibs / gcd) * obs
}

/// Calculate the buffer size appropriate for this loop iteration, respecting
/// a count=N if present.
fn calc_loop_bsize(
    count: &Option<CountType>,
    rstat: &ReadStat,
    wstat: &WriteStat,
    ibs: usize,
    ideal_bsize: usize,
) -> usize {
    match count {
        Some(CountType::Reads(rmax)) => {
            let rmax: u64 = (*rmax).try_into().unwrap();
            let rsofar = rstat.reads_complete + rstat.reads_partial;
            let rremain: usize = (rmax - rsofar).try_into().unwrap();
            cmp::min(ideal_bsize, rremain * ibs)
        }
        Some(CountType::Bytes(bmax)) => {
            let bmax: u128 = (*bmax).try_into().unwrap();
            let bremain: usize = (bmax - wstat.bytes_total).try_into().unwrap();
            cmp::min(ideal_bsize, bremain)
        }
        None => ideal_bsize,
    }
}

/// Decide if the current progress is below a count=N limit or return
/// true if no such limit is set.
fn below_count_limit(count: &Option<CountType>, rstat: &ReadStat, wstat: &WriteStat) -> bool {
    match count {
        Some(CountType::Reads(n)) => {
            let n = (*n).try_into().unwrap();
            // debug_assert!(rstat.reads_complete + rstat.reads_partial >= n);
            rstat.reads_complete + rstat.reads_partial <= n
        }
        Some(CountType::Bytes(n)) => {
            let n = (*n).try_into().unwrap();
            // debug_assert!(wstat.bytes_total >= n);
            wstat.bytes_total <= n
        }
        None => true,
    }
}

/// Perform the copy/convert operations. Stdout version
// Note: Some of dd's functionality depends on whether the output is actually a file. This breaks the Output<Write> abstraction,
// and should be fixed in the future.
fn dd_stdout<R: Read>(mut i: Input<R>, mut o: Output<io::Stdout>) -> Result<(), Box<dyn Error>> {
    let mut rstat = ReadStat {
        reads_complete: 0,
        reads_partial: 0,
        records_truncated: 0,
    };
    let mut wstat = WriteStat {
        writes_complete: 0,
        writes_partial: 0,
        bytes_total: 0,
    };
    let start = time::Instant::now();
    let bsize = calc_bsize(i.ibs, o.obs);

    let prog_tx = {
        let (tx, rx) = mpsc::channel();
        thread::spawn(gen_prog_updater(rx, i.xfer_stats));
        tx
    };

    while below_count_limit(&i.count, &rstat, &wstat) {
        // Read/Write
        let loop_bsize = calc_loop_bsize(&i.count, &rstat, &wstat, i.ibs, bsize);
        match read_helper(&mut i, loop_bsize)? {
            (
                ReadStat {
                    reads_complete: 0,
                    reads_partial: 0,
                    ..
                },
                _,
            ) => break,
            (rstat_update, buf) => {
                let wstat_update = o.write_blocks(buf)?;

                rstat += rstat_update;
                wstat += wstat_update;
            }
        };
        // Update Prog
        prog_tx.send(ProgUpdate {
            reads_complete: rstat.reads_complete,
            reads_partial: rstat.reads_partial,
            writes_complete: wstat.writes_complete,
            writes_partial: wstat.writes_partial,
            bytes_total: wstat.bytes_total,
            records_truncated: rstat.records_truncated,
            duration: start.elapsed(),
        })?;
    }

    if o.cflags.fsync {
        o.fsync()?;
    } else if o.cflags.fdatasync {
        o.fdatasync()?;
    }

    match i.xfer_stats {
        Some(StatusLevel::Noxfer) | Some(StatusLevel::None) => {}
        _ => print_xfer_stats(&ProgUpdate {
            reads_complete: rstat.reads_complete,
            reads_partial: rstat.reads_partial,
            writes_complete: wstat.writes_complete,
            writes_partial: wstat.writes_partial,
            bytes_total: wstat.bytes_total,
            records_truncated: rstat.records_truncated,
            duration: start.elapsed(),
        }),
    }
    Ok(())
}

/// Perform the copy/convert operations. File backed output version
// Note: Some of dd's functionality depends on whether the output is actually a file. This breaks the Output<Write> abstraction,
// and should be fixed in the future.
fn dd_fileout<R: Read>(mut i: Input<R>, mut o: Output<File>) -> Result<(), Box<dyn Error>> {
    let mut rstat = ReadStat {
        reads_complete: 0,
        reads_partial: 0,
        records_truncated: 0,
    };
    let mut wstat = WriteStat {
        writes_complete: 0,
        writes_partial: 0,
        bytes_total: 0,
    };
    let start = time::Instant::now();
    let bsize = calc_bsize(i.ibs, o.obs);

    let prog_tx = {
        let (tx, rx) = mpsc::channel();
        thread::spawn(gen_prog_updater(rx, i.xfer_stats));
        tx
    };

    while below_count_limit(&i.count, &rstat, &wstat) {
        // Read/Write
        let loop_bsize = calc_loop_bsize(&i.count, &rstat, &wstat, i.ibs, bsize);
        match read_helper(&mut i, loop_bsize)? {
            (
                ReadStat {
                    reads_complete: 0,
                    reads_partial: 0,
                    ..
                },
                _,
            ) => break,
            (rstat_update, buf) => {
                let wstat_update = o.write_blocks(buf)?;

                rstat += rstat_update;
                wstat += wstat_update;
            }
        };
        // Update Prog
        prog_tx.send(ProgUpdate {
            reads_complete: rstat.reads_complete,
            reads_partial: rstat.reads_partial,
            writes_complete: wstat.writes_complete,
            writes_partial: wstat.writes_partial,
            bytes_total: wstat.bytes_total,
            records_truncated: rstat.records_truncated,
            duration: start.elapsed(),
        })?;
    }

    if o.cflags.fsync {
        o.fsync()?;
    } else if o.cflags.fdatasync {
        o.fdatasync()?;
    }

    match i.xfer_stats {
        Some(StatusLevel::Noxfer) | Some(StatusLevel::None) => {}
        _ => print_xfer_stats(&ProgUpdate {
            reads_complete: rstat.reads_complete,
            reads_partial: rstat.reads_partial,
            writes_complete: wstat.writes_complete,
            writes_partial: wstat.writes_partial,
            bytes_total: wstat.bytes_total,
            records_truncated: rstat.records_truncated,
            duration: start.elapsed(),
        }),
    }
    Ok(())
}

// The compiler does not like Clippy's suggestion to use &str in place of &String here.
#[allow(clippy::ptr_arg)]
fn append_dashes_if_not_present(mut acc: Vec<String>, s: &String) -> Vec<String> {
    if Some("--") != s.get(0..=1) {
        acc.push(format!("--{}", s));
    }
    acc
}

macro_rules! unpack_or_rtn (
    ($i:expr, $o:expr) =>
    {{
        match ($i, $o)
        {
            (Ok(i), Ok(o)) =>
                (i,o),
            (Err(e), _) =>
            {
                eprintln!("dd Error: {}", e);
                return RTN_FAILURE;
            },
            (_, Err(e)) =>
            {
                eprintln!("dd Error: {}", e);
                return RTN_FAILURE;
            },
        }
    }};
);

pub fn uumain(args: impl uucore::Args) -> i32 {
    let dashed_args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any()
        .iter()
        .fold(Vec::new(), append_dashes_if_not_present);

    let matches = uu_app()
        // TODO: usage, after_help
        //.usage(...)
        //.after_help(...)
        .get_matches_from(dashed_args);

    let result = match (
        matches.is_present(options::INFILE),
        matches.is_present(options::OUTFILE),
    ) {
        (true, true) => {
            let (i, o) =
                unpack_or_rtn!(Input::<File>::new(&matches), Output::<File>::new(&matches));

            dd_fileout(i, o)
        }
        (false, true) => {
            let (i, o) = unpack_or_rtn!(
                Input::<io::Stdin>::new(&matches),
                Output::<File>::new(&matches)
            );

            dd_fileout(i, o)
        }
        (true, false) => {
            let (i, o) = unpack_or_rtn!(
                Input::<File>::new(&matches),
                Output::<io::Stdout>::new(&matches)
            );

            dd_stdout(i, o)
        }
        (false, false) => {
            let (i, o) = unpack_or_rtn!(
                Input::<io::Stdin>::new(&matches),
                Output::<io::Stdout>::new(&matches)
            );

            dd_stdout(i, o)
        }
    };
    match result {
        Ok(_) => RTN_SUCCESS,
        Err(e) => {
            debug_println!("dd exiting with error:\n\t{}", e);
            RTN_FAILURE
        }
    }
}

pub fn uu_app() -> clap::App<'static, 'static> {
    clap::App::new(executable!())
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            clap::Arg::with_name(options::INFILE)
                .long(options::INFILE)
                .takes_value(true)
                .help("if=FILE (alternatively --if FILE) specifies the file used for input. When not specified, stdin is used instead")
        )
        .arg(
            clap::Arg::with_name(options::OUTFILE)
                .long(options::OUTFILE)
                .takes_value(true)
                .help("of=FILE (alternatively --of FILE) specifies the file used for output. When not specified, stdout is used instead")
        )
        .arg(
            clap::Arg::with_name(options::IBS)
                .long(options::IBS)
                .takes_value(true)
                .help("ibs=N (alternatively --ibs N) specifies the size of buffer used for reads (default: 512). Multiplier strings permitted.")
        )
        .arg(
            clap::Arg::with_name(options::OBS)
                .long(options::OBS)
                .takes_value(true)
                .help("obs=N (alternatively --obs N) specifies the size of buffer used for writes (default: 512). Multiplier strings permitted.")
        )
        .arg(
            clap::Arg::with_name(options::BS)
                .long(options::BS)
                .takes_value(true)
                .help("bs=N (alternatively --bs N) specifies ibs=N and obs=N (default: 512). If ibs or obs are also specified, bs=N takes precedence. Multiplier strings permitted.")
        )
        .arg(
            clap::Arg::with_name(options::CBS)
                .long(options::CBS)
                .takes_value(true)
                .help("cbs=BYTES (alternatively --cbs BYTES) specifies the 'conversion block size' in bytes. Applies to the conv=block, and conv=unblock operations. Multiplier strings permitted.")
        )
        .arg(
            clap::Arg::with_name(options::SKIP)
                .long(options::SKIP)
                .takes_value(true)
                .help("skip=N (alternatively --skip N) causes N ibs-sized records of input to be skipped before beginning copy/convert operations. See iflag=count_bytes if skipping N bytes is preferred. Multiplier strings permitted.")
        )
        .arg(
            clap::Arg::with_name(options::SEEK)
                .long(options::SEEK)
                .takes_value(true)
                .help("seek=N (alternatively --seek N) seeks N obs-sized records into output before beginning copy/convert operations. See oflag=seek_bytes if seeking N bytes is preferred. Multiplier strings permitted.")
        )
        .arg(
            clap::Arg::with_name(options::COUNT)
                .long(options::COUNT)
                .takes_value(true)
                .help("count=N (alternatively --count N) stop reading input after N ibs-sized read operations rather than proceeding until EOF. See iflag=count_bytes if stopping after N bytes is preferred. Multiplier strings permitted.")
        )
        .arg(
            clap::Arg::with_name(options::STATUS)
                .long(options::STATUS)
                .takes_value(true)
                .help("status=LEVEL (alternatively --status LEVEL) controls whether volume and performance stats are written to stderr.

When unspecified, dd will print stats upon completion. An example is below.
\t6+0 records in
\t16+0 records out
\t8192 bytes (8.2 kB, 8.0 KiB) copied, 0.00057009 s, 14.4 MB/s
The first two lines are the 'volume' stats and the final line is the 'performance' stats.
The volume stats indicate the number of complete and partial ibs-sized reads, or obs-sized writes that took place during the copy. The format of the volume stats is <complete>+<partial>. If records have been truncated (see conv=block), the volume stats will contain the number of truncated records.

Permissible LEVEL values are:
\t progress: Print periodic performance stats as the copy proceeds.
\t noxfer: Print final volume stats, but not performance stats.
\t none: Do not print any stats.

Printing performance stats is also triggered by the INFO signal (where supported), or the USR1 signal. Setting the POSIXLY_CORRECT environment variable to any value (including an empty value) will cause the USR1 signal to be ignored.

")
        )
        .arg(
            clap::Arg::with_name(options::CONV)
                .long(options::CONV)
                .takes_value(true)
                .help("conv=CONV[,CONV] (alternatively --conv CONV[,CONV]) specifies a comma-separated list of conversion options or (for legacy reasons) file-flags. Conversion options and file flags may be intermixed.

Conversion options:
\t One of {ascii, ebcdic, ibm} will perform an encoding conversion.
\t\t 'ascii' converts from EBCDIC to ASCII. This is the inverse of the 'ebcdic' option.
\t\t 'ebcdic' converts from ASCII to EBCDIC. This is the inverse of the 'ascii' option.
\t\t 'ibm' converts from ASCII to EBCDIC, applying the conventions for '[', ']' and '~' specified in POSIX.

\t One of {ucase, lcase} will perform a case conversion. Works in conjunction with option {ascii, ebcdic, ibm} to infer input encoding. If no other conversion option is specified, input is assumed to be ascii.
\t\t 'ucase' converts from lower-case to upper-case
\t\t 'lcase' converts from upper-case to lower-case.

\t One of {block, unblock}. Convert between lines terminated by newline characters, and fixed-width lines padded by spaces (without any newlines). Both the 'block' and 'unblock' options require cbs=BYTES be specified.
\t\t 'block' for each newline less than the size indicated by cbs=BYTES, remove the newline and pad with spaces up to cbs. Lines longer than cbs are truncated.
\t\t 'unblock' for each block of input of the size indicated by cbs=BYTES, remove right-trailing spaces and replace with a newline character.

\t 'sparse' attempts to seek the output when an obs-sized block consists of only zeros.
\t 'swab' swaps each adjacent pair of bytes. If an odd number of bytes is present, the final byte is omitted.
\t 'sync' pad each ibs-sided block with zeros. If 'block' or 'unblock' is specified, pad with spaces instead.

Flags:
\t One of {excl, nocreat}
\t\t 'excl' the output file must be created. Fail if the output file is already present.
\t\t 'nocreat' the output file will not be created. Fail if the output file in not already present.
\t 'notrunc' the output file will not be truncated. If this option is not present, output will be truncated when opened.
\t 'noerror' all read errors will be ignored. If this option is not present, dd will only ignore Error::Interrupted.
\t 'fdatasync' data will be written before finishing.
\t 'fsync' data and metadata will be written before finishing.

")
        )
        .arg(
            clap::Arg::with_name(options::IFLAG)
                .long(options::IFLAG)
                .takes_value(true)
                .help("iflag=FLAG[,FLAG] (alternatively --iflag FLAG[,FLAG]) a comma separated list of input flags which specify how the input source is treated. FLAG may be any of the input-flags or general-flags specified below.

Input-Flags
\t 'count_bytes' a value to count=N will be interpreted as bytes.
\t 'skip_bytes' a value to skip=N will be interpreted as bytes.
\t 'fullblock' wait for ibs bytes from each read. zero-length reads are still considered EOF.

General-Flags
\t 'direct' use direct I/O for data.
\t 'directory' fail unless the given input (if used as an iflag) or output (if used as an oflag) is a directory.
\t 'dsync' use synchronized I/O for data.
\t 'sync' use synchronized I/O for data and metadata.
\t 'nonblock' use non-blocking I/O.
\t 'noatime' do not update access time.
\t 'nocache' request that OS drop cache.
\t 'noctty' do not assign a controlling tty.
\t 'nofollow' do not follow system links.

Output-Flags
\t 'append' open file in append mode. Consider setting conv=notrunc as well.
\t 'seek_bytes' a value to seek=N will be interpreted as bytes.

")
        )
        .arg(
            clap::Arg::with_name(options::OFLAG)
                .long(options::OFLAG)
                .takes_value(true)
                .help("oflag=FLAG[,FLAG] (alternatively --oflag FLAG[,FLAG]) a comma separated list of output flags which specify how the output source is treated. FLAG may be any of the output-flags or general-flags specified below.

Output-Flags
\t 'append' open file in append mode. Consider setting conv=notrunc as well.
\t 'seek_bytes' a value to seek=N will be interpreted as bytes.

General-Flags
\t 'direct' use direct I/O for data.
\t 'directory' fail unless the given input (if used as an iflag) or output (if used as an oflag) is a directory.
\t 'dsync' use synchronized I/O for data.
\t 'sync' use synchronized I/O for data and metadata.
\t 'nonblock' use non-blocking I/O.
\t 'noatime' do not update access time.
\t 'nocache' request that OS drop cache.
\t 'noctty' do not assign a controlling tty.
\t 'nofollow' do not follow system links.

")
        )
}
