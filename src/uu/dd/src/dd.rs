// This file is part of the uutils coreutils package.
//
// (c) Tyler Steele <tyler.steele@protonmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore fname, tname, fpath, specfile, testfile, unspec, ifile, ofile, outfile, fullblock, urand, fileio, atoe, atoibm, behaviour, bmax, bremain, btotal, cflags, creat, ctable, ctty, datastructures, doesnt, etoa, fileout, fname, gnudd, iconvflags, nocache, noctty, noerror, nofollow, nolinks, nonblock, oconvflags, outfile, parseargs, rlen, rmax, rposition, rremain, rsofar, rstat, sigusr, sigval, wlen, wstat

#[cfg(test)]
mod dd_unit_tests;

mod datastructures;
use datastructures::*;

mod parseargs;
use parseargs::Matches;

mod conversion_tables;
use conversion_tables::*;

use std::cmp;
use std::convert::TryInto;
use std::env;
#[cfg(target_os = "linux")]
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, Write};
#[cfg(target_os = "linux")]
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::mpsc;
#[cfg(target_os = "linux")]
use std::sync::{atomic::AtomicUsize, atomic::Ordering, Arc};
use std::thread;
use std::time;

use byte_unit::Byte;
use clap::{self, crate_version};
use gcd::Gcd;
#[cfg(target_os = "linux")]
use signal_hook::consts::signal;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::InvalidEncodingHandling;

const ABOUT: &str = "copy, and optionally convert, a file system resource";
const BUF_INIT_BYTE: u8 = 0xDD;
const NEWLINE: u8 = b'\n';
const SPACE: u8 = b' ';

struct Input<R: Read> {
    src: R,
    non_ascii: bool,
    ibs: usize,
    print_level: Option<StatusLevel>,
    count: Option<CountType>,
    cflags: IConvFlags,
    iflags: IFlags,
}

impl Input<io::Stdin> {
    fn new(matches: &Matches) -> UResult<Self> {
        let ibs = parseargs::parse_ibs(matches)?;
        let non_ascii = parseargs::parse_input_non_ascii(matches)?;
        let print_level = parseargs::parse_status_level(matches)?;
        let cflags = parseargs::parse_conv_flag_input(matches)?;
        let iflags = parseargs::parse_iflags(matches)?;
        let skip = parseargs::parse_skip_amt(&ibs, &iflags, matches)?;
        let count = parseargs::parse_count(&iflags, matches)?;

        let mut i = Input {
            src: io::stdin(),
            non_ascii,
            ibs,
            print_level,
            count,
            cflags,
            iflags,
        };

        if let Some(amt) = skip {
            let mut buf = vec![BUF_INIT_BYTE; amt];
            i.force_fill(&mut buf, amt)
                .map_err_context(|| "failed to read input".to_string())?;
        }

        Ok(i)
    }
}

#[cfg(target_os = "linux")]
fn make_linux_iflags(iflags: &IFlags) -> Option<libc::c_int> {
    let mut flag = 0;

    if iflags.direct {
        flag |= libc::O_DIRECT;
    }
    if iflags.directory {
        flag |= libc::O_DIRECTORY;
    }
    if iflags.dsync {
        flag |= libc::O_DSYNC;
    }
    if iflags.noatime {
        flag |= libc::O_NOATIME;
    }
    if iflags.noctty {
        flag |= libc::O_NOCTTY;
    }
    if iflags.nofollow {
        flag |= libc::O_NOFOLLOW;
    }
    if iflags.nonblock {
        flag |= libc::O_NONBLOCK;
    }
    if iflags.sync {
        flag |= libc::O_SYNC;
    }

    if flag != 0 {
        Some(flag)
    } else {
        None
    }
}

impl Input<File> {
    fn new(matches: &Matches) -> UResult<Self> {
        let ibs = parseargs::parse_ibs(matches)?;
        let non_ascii = parseargs::parse_input_non_ascii(matches)?;
        let print_level = parseargs::parse_status_level(matches)?;
        let cflags = parseargs::parse_conv_flag_input(matches)?;
        let iflags = parseargs::parse_iflags(matches)?;
        let skip = parseargs::parse_skip_amt(&ibs, &iflags, matches)?;
        let count = parseargs::parse_count(&iflags, matches)?;

        if let Some(fname) = matches.value_of(options::INFILE) {
            let mut src = {
                let mut opts = OpenOptions::new();
                opts.read(true);

                #[cfg(target_os = "linux")]
                if let Some(libc_flags) = make_linux_iflags(&iflags) {
                    opts.custom_flags(libc_flags);
                }

                opts.open(fname)
                    .map_err_context(|| "failed to open input file".to_string())?
            };

            if let Some(amt) = skip {
                let amt: u64 = amt
                    .try_into()
                    .map_err(|_| USimpleError::new(1, "failed to parse seek amount"))?;
                src.seek(io::SeekFrom::Start(amt))
                    .map_err_context(|| "failed to seek in input file".to_string())?;
            }

            let i = Input {
                src,
                non_ascii,
                ibs,
                print_level,
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
        let target_len = buf.len();
        loop {
            match self.src.read(&mut buf[base_idx..]) {
                Ok(0) => return Ok(base_idx),
                Ok(rlen) if self.iflags.fullblock => {
                    base_idx += rlen;

                    if base_idx >= target_len {
                        return Ok(target_len);
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
    fn fill_consecutive(&mut self, buf: &mut Vec<u8>) -> std::io::Result<ReadStat> {
        let mut reads_complete = 0;
        let mut reads_partial = 0;
        let mut bytes_total = 0;

        for chunk in buf.chunks_mut(self.ibs) {
            match self.read(chunk)? {
                rlen if rlen == self.ibs => {
                    bytes_total += rlen;
                    reads_complete += 1;
                }
                rlen if rlen > 0 => {
                    bytes_total += rlen;
                    reads_partial += 1;
                }
                _ => break,
            }
        }

        buf.truncate(bytes_total);
        Ok(ReadStat {
            reads_complete,
            reads_partial,
            // Records are not truncated when filling.
            records_truncated: 0,
        })
    }

    /// Fills a given buffer.
    /// Reads in increments of 'self.ibs'.
    /// The start of each ibs-sized read is aligned to multiples of ibs; remaining space is filled with the 'pad' byte.
    fn fill_blocks(&mut self, buf: &mut Vec<u8>, pad: u8) -> std::io::Result<ReadStat> {
        let mut reads_complete = 0;
        let mut reads_partial = 0;
        let mut base_idx = 0;

        while base_idx < buf.len() {
            let next_blk = cmp::min(base_idx + self.ibs, buf.len());
            let target_len = next_blk - base_idx;

            match self.read(&mut buf[base_idx..next_blk])? {
                0 => break,
                rlen if rlen < target_len => {
                    reads_partial += 1;
                    let padding = vec![pad; target_len - rlen];
                    buf.splice(base_idx + rlen..next_blk, padding.into_iter());
                }
                _ => {
                    reads_complete += 1;
                }
            }

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
    fn force_fill(&mut self, buf: &mut [u8], target_len: usize) -> std::io::Result<usize> {
        let mut base_idx = 0;
        while base_idx < target_len {
            base_idx += self.read(&mut buf[base_idx..target_len])?;
        }

        Ok(base_idx)
    }
}

trait OutputTrait: Sized + Write {
    fn new(matches: &Matches) -> UResult<Self>;
    fn fsync(&mut self) -> io::Result<()>;
    fn fdatasync(&mut self) -> io::Result<()>;
}

struct Output<W: Write> {
    dst: W,
    obs: usize,
    cflags: OConvFlags,
}

impl OutputTrait for Output<io::Stdout> {
    fn new(matches: &Matches) -> UResult<Self> {
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

impl<W: Write> Output<W>
where
    Self: OutputTrait,
{
    fn write_blocks(&mut self, buf: Vec<u8>) -> io::Result<WriteStat> {
        let mut writes_complete = 0;
        let mut writes_partial = 0;
        let mut bytes_total = 0;

        for chunk in buf.chunks(self.obs) {
            match self.write(chunk)? {
                wlen if wlen < chunk.len() => {
                    writes_partial += 1;
                    bytes_total += wlen;
                }
                wlen => {
                    writes_complete += 1;
                    bytes_total += wlen;
                }
            }
        }

        Ok(WriteStat {
            writes_complete,
            writes_partial,
            bytes_total: bytes_total.try_into().unwrap_or(0u128),
        })
    }

    fn dd_out<R: Read>(mut self, mut i: Input<R>) -> UResult<()> {
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
        let bsize = calc_bsize(i.ibs, self.obs);

        let prog_tx = {
            let (tx, rx) = mpsc::channel();
            thread::spawn(gen_prog_updater(rx, i.print_level));
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
                    let wstat_update = self
                        .write_blocks(buf)
                        .map_err_context(|| "failed to write output".to_string())?;

                    rstat += rstat_update;
                    wstat += wstat_update;
                }
            };
            // Update Prog
            prog_tx
                .send(ProgUpdate {
                    read_stat: rstat,
                    write_stat: wstat,
                    duration: start.elapsed(),
                })
                .map_err(|_| USimpleError::new(1, "failed to write output"))?;
        }

        if self.cflags.fsync {
            self.fsync()
                .map_err_context(|| "failed to write output".to_string())?;
        } else if self.cflags.fdatasync {
            self.fdatasync()
                .map_err_context(|| "failed to write output".to_string())?;
        }

        match i.print_level {
            Some(StatusLevel::Noxfer) | Some(StatusLevel::None) => {}
            _ => print_transfer_stats(&ProgUpdate {
                read_stat: rstat,
                write_stat: wstat,
                duration: start.elapsed(),
            }),
        }
        Ok(())
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

impl OutputTrait for Output<File> {
    fn new(matches: &Matches) -> UResult<Self> {
        fn open_dst(path: &Path, cflags: &OConvFlags, oflags: &OFlags) -> Result<File, io::Error> {
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

            opts.open(path)
        }
        let obs = parseargs::parse_obs(matches)?;
        let cflags = parseargs::parse_conv_flag_output(matches)?;
        let oflags = parseargs::parse_oflags(matches)?;
        let seek = parseargs::parse_seek_amt(&obs, &oflags, matches)?;

        if let Some(fname) = matches.value_of(options::OUTFILE) {
            let mut dst = open_dst(Path::new(&fname), &cflags, &oflags)
                .map_err_context(|| format!("failed to open {}", fname.quote()))?;

            if let Some(amt) = seek {
                let amt: u64 = amt
                    .try_into()
                    .map_err(|_| USimpleError::new(1, "failed to parse seek amount"))?;
                dst.seek(io::SeekFrom::Start(amt))
                    .map_err_context(|| "failed to seek in output file".to_string())?;
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

/// Splits the content of buf into cbs-length blocks
/// Appends padding as specified by conv=block and cbs=N
/// Expects ascii encoded data
fn block(buf: Vec<u8>, cbs: usize, rstat: &mut ReadStat) -> Vec<Vec<u8>> {
    let mut blocks = buf
        .split(|&e| e == NEWLINE)
        .map(|split| split.to_vec())
        .fold(Vec::new(), |mut blocks, mut split| {
            if split.len() > cbs {
                rstat.records_truncated += 1;
            }
            split.resize(cbs, SPACE);
            blocks.push(split);

            blocks
        });

    if let Some(last) = blocks.last() {
        if last.iter().all(|&e| e == SPACE) {
            blocks.pop();
        }
    }

    blocks
}

/// Trims padding from each cbs-length partition of buf
/// as specified by conv=unblock and cbs=N
/// Expects ascii encoded data
fn unblock(buf: Vec<u8>, cbs: usize) -> Vec<u8> {
    buf.chunks(cbs).fold(Vec::new(), |mut acc, block| {
        if let Some(last_char_idx) = block.iter().rposition(|&e| e != SPACE) {
            // Include text up to last space.
            acc.extend(&block[..=last_char_idx]);
        }

        acc.push(NEWLINE);
        acc
    })
}

/// A helper for teasing out which options must be applied and in which order.
/// Some user options, such as the presence of conversion tables, will determine whether the input is assumed to be ascii. The parser sets the Input::non_ascii flag accordingly.
/// Examples:
///     - If conv=ebcdic or conv=ibm is specified then block, unblock or swab must be performed before the conversion happens since the source will start in ascii.
///     - If conv=ascii is specified then block, unblock or swab must be performed after the conversion since the source starts in ebcdic.
///     - If no conversion is specified then the source is assumed to be in ascii.
/// For more info see `info dd`
fn conv_block_unblock_helper<R: Read>(
    mut buf: Vec<u8>,
    i: &mut Input<R>,
    rstat: &mut ReadStat,
) -> Result<Vec<u8>, InternalError> {
    // Local Predicate Fns -------------------------------------------------
    fn should_block_then_conv<R: Read>(i: &Input<R>) -> bool {
        !i.non_ascii && i.cflags.block.is_some()
    }
    fn should_conv_then_block<R: Read>(i: &Input<R>) -> bool {
        i.non_ascii && i.cflags.block.is_some()
    }
    fn should_unblock_then_conv<R: Read>(i: &Input<R>) -> bool {
        !i.non_ascii && i.cflags.unblock.is_some()
    }
    fn should_conv_then_unblock<R: Read>(i: &Input<R>) -> bool {
        i.non_ascii && i.cflags.unblock.is_some()
    }
    fn conv_only<R: Read>(i: &Input<R>) -> bool {
        i.cflags.ctable.is_some() && i.cflags.block.is_none() && i.cflags.unblock.is_none()
    }
    // Local Helper Fns ----------------------------------------------------
    fn apply_conversion(buf: &mut [u8], ct: &ConversionTable) {
        for idx in 0..buf.len() {
            buf[idx] = ct[buf[idx] as usize];
        }
    }
    // --------------------------------------------------------------------
    if conv_only(i) {
        // no block/unblock
        let ct = i.cflags.ctable.unwrap();
        apply_conversion(&mut buf, ct);

        Ok(buf)
    } else if should_block_then_conv(i) {
        // ascii input so perform the block first
        let cbs = i.cflags.block.unwrap();

        let mut blocks = block(buf, cbs, rstat);

        if let Some(ct) = i.cflags.ctable {
            for buf in blocks.iter_mut() {
                apply_conversion(buf, ct);
            }
        }

        let blocks = blocks.into_iter().flatten().collect();

        Ok(blocks)
    } else if should_conv_then_block(i) {
        // Non-ascii so perform the conversion first
        let cbs = i.cflags.block.unwrap();

        if let Some(ct) = i.cflags.ctable {
            apply_conversion(&mut buf, ct);
        }

        let blocks = block(buf, cbs, rstat).into_iter().flatten().collect();

        Ok(blocks)
    } else if should_unblock_then_conv(i) {
        // ascii input so perform the unblock first
        let cbs = i.cflags.unblock.unwrap();

        let mut buf = unblock(buf, cbs);

        if let Some(ct) = i.cflags.ctable {
            apply_conversion(&mut buf, ct);
        }

        Ok(buf)
    } else if should_conv_then_unblock(i) {
        // Non-ascii input so perform the conversion first
        let cbs = i.cflags.unblock.unwrap();

        if let Some(ct) = i.cflags.ctable {
            apply_conversion(&mut buf, ct);
        }

        let buf = unblock(buf, cbs);

        Ok(buf)
    } else {
        // The following error should not happen, as it results from
        // insufficient command line data. This case should be caught
        // by the parser before making it this far.
        // Producing this error is an alternative to risking an unwrap call
        // on 'cbs' if the required data is not provided.
        Err(InternalError::InvalidConvBlockUnblockCase)
    }
}

/// Read helper performs read operations common to all dd reads, and dispatches the buffer to relevant helper functions as dictated by the operations requested by the user.
fn read_helper<R: Read>(i: &mut Input<R>, bsize: usize) -> UResult<(ReadStat, Vec<u8>)> {
    // Local Predicate Fns -----------------------------------------------
    fn is_conv<R: Read>(i: &Input<R>) -> bool {
        i.cflags.ctable.is_some()
    }
    fn is_block<R: Read>(i: &Input<R>) -> bool {
        i.cflags.block.is_some()
    }
    fn is_unblock<R: Read>(i: &Input<R>) -> bool {
        i.cflags.unblock.is_some()
    }
    // Local Helper Fns -------------------------------------------------
    fn perform_swab(buf: &mut [u8]) {
        for base in (1..buf.len()).step_by(2) {
            buf.swap(base, base - 1);
        }
    }
    // ------------------------------------------------------------------
    // Read
    let mut buf = vec![BUF_INIT_BYTE; bsize];
    let mut rstat = match i.cflags.sync {
        Some(ch) => i
            .fill_blocks(&mut buf, ch)
            .map_err_context(|| "failed to write output".to_string())?,
        _ => i
            .fill_consecutive(&mut buf)
            .map_err_context(|| "failed to write output".to_string())?,
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

// Print io lines of a status update:
// <complete>+<partial> records in
// <complete>+<partial> records out
fn print_io_lines(update: &ProgUpdate) {
    eprintln!(
        "{}+{} records in",
        update.read_stat.reads_complete, update.read_stat.reads_partial
    );
    if update.read_stat.records_truncated > 0 {
        eprintln!("{} truncated records", update.read_stat.records_truncated);
    }
    eprintln!(
        "{}+{} records out",
        update.write_stat.writes_complete, update.write_stat.writes_partial
    );
}
// Print the progress line of a status update:
// <byte-count> bytes (<base-1000-size>, <base-2-size>) copied, <time> s, <base-2-rate>/s
fn make_prog_line(update: &ProgUpdate) -> String {
    let btotal_metric = Byte::from_bytes(update.write_stat.bytes_total)
        .get_appropriate_unit(false)
        .format(0);
    let btotal_bin = Byte::from_bytes(update.write_stat.bytes_total)
        .get_appropriate_unit(true)
        .format(0);
    let safe_millis = cmp::max(1, update.duration.as_millis());
    let transfer_rate = Byte::from_bytes(1000 * (update.write_stat.bytes_total / safe_millis))
        .get_appropriate_unit(false)
        .format(1);

    format!(
        "{} bytes ({}, {}) copied, {:.1} s, {}/s",
        update.write_stat.bytes_total,
        btotal_metric,
        btotal_bin,
        update.duration.as_secs_f64(),
        transfer_rate
    )
}
// Print progress line only. Overwrite the current line.
fn reprint_prog_line(update: &ProgUpdate) {
    eprint!("\r{}", make_prog_line(update));
}
// Print progress line only. Print as a new line.
fn print_prog_line(update: &ProgUpdate) {
    eprintln!("{}", make_prog_line(update));
}
// Print both io lines and progress line.
fn print_transfer_stats(update: &ProgUpdate) {
    print_io_lines(update);
    print_prog_line(update);
}

// Generate a progress updater that tracks progress, receives updates, and responds to progress update requests (signals).
// Signals:
// - SIGUSR1: Trigger progress line reprint. Linux (GNU & BSD) only.
// - TODO: SIGINFO: Trigger progress line reprint. BSD-style Linux only.
fn gen_prog_updater(rx: mpsc::Receiver<ProgUpdate>, print_level: Option<StatusLevel>) -> impl Fn() {
    // --------------------------------------------------------------
    #[cfg(target_os = "linux")]
    const SIGUSR1_USIZE: usize = signal::SIGUSR1 as usize;
    // --------------------------------------------------------------
    #[cfg(target_os = "linux")]
    fn posixly_correct() -> bool {
        env::var("POSIXLY_CORRECT").is_ok()
    }
    #[cfg(target_os = "linux")]
    fn register_linux_signal_handler(sigval: Arc<AtomicUsize>) -> Result<(), Box<dyn Error>> {
        if !posixly_correct() {
            signal_hook::flag::register_usize(signal::SIGUSR1, sigval, SIGUSR1_USIZE)?;
        }

        Ok(())
    }
    // --------------------------------------------------------------
    move || {
        #[cfg(target_os = "linux")]
        let sigval = Arc::new(AtomicUsize::new(0));

        #[cfg(target_os = "linux")]
        register_linux_signal_handler(sigval.clone()).unwrap_or_else(|e| {
            if Some(StatusLevel::None) != print_level {
                eprintln!(
                    "Internal dd Warning: Unable to register signal handler \n\t{}",
                    e
                );
            }
        });

        while let Ok(update) = rx.recv() {
            // (Re)print status line if progress is requested.
            if Some(StatusLevel::Progress) == print_level {
                reprint_prog_line(&update);
            }
            // Handle signals
            #[cfg(target_os = "linux")]
            if let SIGUSR1_USIZE = sigval.load(Ordering::Relaxed) {
                print_transfer_stats(&update);
            };
        }
    }
}

// Calculate a 'good' internal buffer size.
// For performance of the read/write functions, the buffer should hold
// both an integral number of reads and an integral number of writes. For
// sane real-world memory use, it should not be too large. I believe
// the least common multiple is a good representation of these interests.
// https://en.wikipedia.org/wiki/Least_common_multiple#Using_the_greatest_common_divisor
fn calc_bsize(ibs: usize, obs: usize) -> usize {
    let gcd = Gcd::gcd(ibs, obs);
    // calculate the lcm from gcd
    (ibs / gcd) * obs
}

// Calculate the buffer size appropriate for this loop iteration, respecting
// a count=N if present.
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

// Decide if the current progress is below a count=N limit or return
// true if no such limit is set.
fn below_count_limit(count: &Option<CountType>, rstat: &ReadStat, wstat: &WriteStat) -> bool {
    match count {
        Some(CountType::Reads(n)) => {
            let n = (*n).try_into().unwrap();
            rstat.reads_complete + rstat.reads_partial <= n
        }
        Some(CountType::Bytes(n)) => {
            let n = (*n).try_into().unwrap();
            wstat.bytes_total <= n
        }
        None => true,
    }
}

fn append_dashes_if_not_present(mut acc: Vec<String>, mut s: String) -> Vec<String> {
    if !s.starts_with("--") && !s.starts_with('-') {
        s.insert_str(0, "--");
    }
    acc.push(s);
    acc
}

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let dashed_args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any()
        .into_iter()
        .fold(Vec::new(), append_dashes_if_not_present);

    let matches = uu_app()
        //.after_help(TODO: Add note about multiplier strings here.)
        .get_matches_from(dashed_args);

    match (
        matches.is_present(options::INFILE),
        matches.is_present(options::OUTFILE),
    ) {
        (true, true) => {
            let i = Input::<File>::new(&matches)?;
            let o = Output::<File>::new(&matches)?;
            o.dd_out(i)
        }
        (false, true) => {
            let i = Input::<io::Stdin>::new(&matches)?;
            let o = Output::<File>::new(&matches)?;
            o.dd_out(i)
        }
        (true, false) => {
            let i = Input::<File>::new(&matches)?;
            let o = Output::<io::Stdout>::new(&matches)?;
            o.dd_out(i)
        }
        (false, false) => {
            let i = Input::<io::Stdin>::new(&matches)?;
            let o = Output::<io::Stdout>::new(&matches)?;
            o.dd_out(i)
        }
    }
}

pub fn uu_app() -> clap::App<'static, 'static> {
    clap::App::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            clap::Arg::with_name(options::INFILE)
                .long(options::INFILE)
                .takes_value(true)
                .require_equals(true)
                .value_name("FILE")
                .help("(alternatively if=FILE) specifies the file used for input. When not specified, stdin is used instead")
        )
        .arg(
            clap::Arg::with_name(options::OUTFILE)
                .long(options::OUTFILE)
                .takes_value(true)
                .require_equals(true)
                .value_name("FILE")
                .help("(alternatively of=FILE) specifies the file used for output. When not specified, stdout is used instead")
        )
        .arg(
            clap::Arg::with_name(options::IBS)
                .long(options::IBS)
                .takes_value(true)
                .require_equals(true)
                .value_name("N")
                .help("(alternatively ibs=N) specifies the size of buffer used for reads (default: 512). Multiplier strings permitted.")
        )
        .arg(
            clap::Arg::with_name(options::OBS)
                .long(options::OBS)
                .takes_value(true)
                .require_equals(true)
                .value_name("N")
                .help("(alternatively obs=N) specifies the size of buffer used for writes (default: 512). Multiplier strings permitted.")
        )
        .arg(
            clap::Arg::with_name(options::BS)
                .long(options::BS)
                .takes_value(true)
                .require_equals(true)
                .value_name("N")
                .help("(alternatively bs=N) specifies ibs=N and obs=N (default: 512). If ibs or obs are also specified, bs=N takes precedence. Multiplier strings permitted.")
        )
        .arg(
            clap::Arg::with_name(options::CBS)
                .long(options::CBS)
                .takes_value(true)
                .require_equals(true)
                .value_name("N")
                .help("(alternatively cbs=BYTES) specifies the 'conversion block size' in bytes. Applies to the conv=block, and conv=unblock operations. Multiplier strings permitted.")
        )
        .arg(
            clap::Arg::with_name(options::SKIP)
                .long(options::SKIP)
                .takes_value(true)
                .require_equals(true)
                .value_name("N")
                .help("(alternatively skip=N) causes N ibs-sized records of input to be skipped before beginning copy/convert operations. See iflag=count_bytes if skipping N bytes is preferred. Multiplier strings permitted.")
        )
        .arg(
            clap::Arg::with_name(options::SEEK)
                .long(options::SEEK)
                .takes_value(true)
                .require_equals(true)
                .value_name("N")
                .help("(alternatively seek=N) seeks N obs-sized records into output before beginning copy/convert operations. See oflag=seek_bytes if seeking N bytes is preferred. Multiplier strings permitted.")
        )
        .arg(
            clap::Arg::with_name(options::COUNT)
                .long(options::COUNT)
                .takes_value(true)
                .require_equals(true)
                .value_name("N")
                .help("(alternatively count=N) stop reading input after N ibs-sized read operations rather than proceeding until EOF. See iflag=count_bytes if stopping after N bytes is preferred. Multiplier strings permitted.")
        )
        .arg(
            clap::Arg::with_name(options::STATUS)
                .long(options::STATUS)
                .takes_value(true)
                .require_equals(true)
                .value_name("LEVEL")
                .help("(alternatively status=LEVEL) controls whether volume and performance stats are written to stderr.

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
                .require_equals(true)
                .value_name("CONV")
                .help("(alternatively conv=CONV[,CONV]) specifies a comma-separated list of conversion options or (for legacy reasons) file flags. Conversion options and file flags may be intermixed.

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

Conversion Flags:
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
                .require_equals(true)
                .value_name("FLAG")
                .help("(alternatively iflag=FLAG[,FLAG]) a comma separated list of input flags which specify how the input source is treated. FLAG may be any of the input-flags or general-flags specified below.

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

")
        )
        .arg(
            clap::Arg::with_name(options::OFLAG)
                .long(options::OFLAG)
                .takes_value(true)
                .require_equals(true)
                .value_name("FLAG")
                .help("(alternatively oflag=FLAG[,FLAG]) a comma separated list of output flags which specify how the output source is treated. FLAG may be any of the output-flags or general-flags specified below.

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
