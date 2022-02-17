// This file is part of the uutils coreutils package.
//
// (c) Tyler Steele <tyler.steele@protonmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore fname, tname, fpath, specfile, testfile, unspec, ifile, ofile, outfile, fullblock, urand, fileio, atoe, atoibm, behaviour, bmax, bremain, btotal, cflags, creat, ctable, ctty, datastructures, doesnt, etoa, fileout, fname, gnudd, iconvflags, nocache, noctty, noerror, nofollow, nolinks, nonblock, oconvflags, outfile, parseargs, rlen, rmax, rposition, rremain, rsofar, rstat, sigusr, sigval, wlen, wstat seekable

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
use clap::{crate_version, App, AppSettings, Arg, ArgMatches};
use gcd::Gcd;
#[cfg(target_os = "linux")]
use signal_hook::consts::signal;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::show_error;
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

        let mut i = Self {
            src: io::stdin(),
            non_ascii,
            ibs,
            print_level,
            count,
            cflags,
            iflags,
        };

        if let Some(amt) = skip {
            let num_bytes_read = i
                .force_fill(amt.try_into().unwrap())
                .map_err_context(|| "failed to read input".to_string())?;
            if num_bytes_read < amt {
                show_error!("'standard input': cannot skip to specified offset");
            }
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

            let i = Self {
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

    /// Read the specified number of bytes from this reader.
    ///
    /// On success, this method returns the number of bytes read. If
    /// this reader has fewer than `n` bytes available, then it reads
    /// as many as possible. In that case, this method returns a
    /// number less than `n`.
    ///
    /// # Errors
    ///
    /// If there is a problem reading.
    fn force_fill(&mut self, n: u64) -> std::io::Result<usize> {
        let mut buf = vec![];
        self.take(n).read_to_end(&mut buf)
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
        let oflags = parseargs::parse_oflags(matches)?;
        let seek = parseargs::parse_seek_amt(&obs, &oflags, matches)?;

        let mut dst = io::stdout();

        // stdout is not seekable, so we just write null bytes.
        if let Some(amt) = seek {
            let bytes = vec![b'\0'; amt];
            dst.write_all(&bytes)
                .map_err_context(|| String::from("write error"))?;
        }

        Ok(Self { dst, obs, cflags })
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
    fn write_blocks(&mut self, buf: &[u8]) -> io::Result<WriteStat> {
        let mut writes_complete = 0;
        let mut writes_partial = 0;
        let mut bytes_total = 0;

        for chunk in buf.chunks(self.obs) {
            let wlen = self.write(chunk)?;
            if wlen < self.obs {
                writes_partial += 1;
            } else {
                writes_complete += 1;
            }
            bytes_total += wlen;
        }

        Ok(WriteStat {
            writes_complete,
            writes_partial,
            bytes_total: bytes_total.try_into().unwrap_or(0u128),
        })
    }

    /// Print the read/write statistics.
    fn print_stats<R: Read>(&self, i: &Input<R>, prog_update: &ProgUpdate) {
        match i.print_level {
            Some(StatusLevel::None) => {}
            Some(StatusLevel::Noxfer) => print_io_lines(prog_update),
            Some(StatusLevel::Progress) | None => print_transfer_stats(prog_update),
        }
    }

    /// Flush the output to disk, if configured to do so.
    fn sync(&mut self) -> std::io::Result<()> {
        if self.cflags.fsync {
            self.fsync()
        } else if self.cflags.fdatasync {
            self.fdatasync()
        } else {
            // Intentionally do nothing in this case.
            Ok(())
        }
    }

    /// Copy the given input data to this output, consuming both.
    ///
    /// This method contains the main loop for the `dd` program. Bytes
    /// are read in blocks from `i` and written in blocks to this
    /// output. Read/write statistics are reported to stderr as
    /// configured by the `status` command-line argument.
    ///
    /// # Errors
    ///
    /// If there is a problem reading from the input or writing to
    /// this output.
    fn dd_out<R: Read>(mut self, mut i: Input<R>) -> std::io::Result<()> {
        // The read and write statistics.
        //
        // These objects are counters, initialized to zero. After each
        // iteration of the main loop, each will be incremented by the
        // number of blocks read and written, respectively.
        let mut rstat = Default::default();
        let mut wstat = Default::default();

        // The time at which the main loop starts executing.
        //
        // When `status=progress` is given on the command-line, the
        // `dd` program reports its progress every second or so. Part
        // of its report includes the throughput in bytes per second,
        // which requires knowing how long the process has been
        // running.
        let start = time::Instant::now();

        // A good buffer size for reading.
        //
        // This is an educated guess about a good buffer size based on
        // the input and output block sizes.
        let bsize = calc_bsize(i.ibs, self.obs);

        // Start a thread that reports transfer progress.
        //
        // When `status=progress` is given on the command-line, the
        // `dd` program reports its progress every second or so. We
        // perform this reporting in a new thread so as not to take
        // any CPU time away from the actual reading and writing of
        // data. We send a `ProgUpdate` from the transmitter `prog_tx`
        // to the receives `rx`, and the receiver prints the transfer
        // information.
        let (prog_tx, rx) = mpsc::channel();
        thread::spawn(gen_prog_updater(rx, i.print_level));

        // The main read/write loop.
        //
        // Each iteration reads blocks from the input and writes
        // blocks to this output. Read/write statistics are updated on
        // each iteration and cumulative statistics are reported to
        // the progress reporting thread.
        while below_count_limit(&i.count, &rstat, &wstat) {
            // Read a block from the input then write the block to the output.
            //
            // As an optimization, make an educated guess about the
            // best buffer size for reading based on the number of
            // blocks already read and the number of blocks remaining.
            let loop_bsize = calc_loop_bsize(&i.count, &rstat, &wstat, i.ibs, bsize);
            let (rstat_update, buf) = read_helper(&mut i, loop_bsize)?;
            if rstat_update.is_empty() {
                break;
            }
            let wstat_update = self.write_blocks(&buf)?;

            // Update the read/write stats and inform the progress thread.
            //
            // If the receiver is disconnected, `send()` returns an
            // error. Since it is just reporting progress and is not
            // crucial to the operation of `dd`, let's just ignore the
            // error.
            rstat += rstat_update;
            wstat += wstat_update;
            let prog_update = ProgUpdate::new(rstat, wstat, start.elapsed());
            prog_tx.send(prog_update).unwrap_or(());
        }

        // Flush the output, if configured to do so.
        self.sync()?;

        // Print the final read/write statistics.
        let prog_update = ProgUpdate::new(rstat, wstat, start.elapsed());
        self.print_stats(&i, &prog_update);
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

            let i = seek.unwrap_or(0).try_into().unwrap();
            if !cflags.notrunc {
                dst.set_len(i)
                    .map_err_context(|| "failed to truncate output file".to_string())?;
            }
            dst.seek(io::SeekFrom::Start(i))
                .map_err_context(|| "failed to seek in output file".to_string())?;

            Ok(Self { dst, obs, cflags })
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
fn block(buf: &[u8], cbs: usize, rstat: &mut ReadStat) -> Vec<Vec<u8>> {
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
fn unblock(buf: &[u8], cbs: usize) -> Vec<u8> {
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

        let mut blocks = block(&buf, cbs, rstat);

        if let Some(ct) = i.cflags.ctable {
            for buf in &mut blocks {
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

        let blocks = block(&buf, cbs, rstat).into_iter().flatten().collect();

        Ok(blocks)
    } else if should_unblock_then_conv(i) {
        // ascii input so perform the unblock first
        let cbs = i.cflags.unblock.unwrap();

        let mut buf = unblock(&buf, cbs);

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

        let buf = unblock(&buf, cbs);

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
fn read_helper<R: Read>(i: &mut Input<R>, bsize: usize) -> std::io::Result<(ReadStat, Vec<u8>)> {
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
        let buf = conv_block_unblock_helper(buf, i, &mut rstat).unwrap();
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

        let mut progress_as_secs = 0;
        while let Ok(update) = rx.recv() {
            // (Re)print status line if progress is requested.
            if Some(StatusLevel::Progress) == print_level
                && update.duration.as_secs() >= progress_as_secs
            {
                reprint_prog_line(&update);
                progress_as_secs = update.duration.as_secs() + 1;
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

#[uucore::main]
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
            o.dd_out(i).map_err_context(|| "IO error".to_string())
        }
        (false, true) => {
            let i = Input::<io::Stdin>::new(&matches)?;
            let o = Output::<File>::new(&matches)?;
            o.dd_out(i).map_err_context(|| "IO error".to_string())
        }
        (true, false) => {
            let i = Input::<File>::new(&matches)?;
            let o = Output::<io::Stdout>::new(&matches)?;
            o.dd_out(i).map_err_context(|| "IO error".to_string())
        }
        (false, false) => {
            let i = Input::<io::Stdin>::new(&matches)?;
            let o = Output::<io::Stdout>::new(&matches)?;
            o.dd_out(i).map_err_context(|| "IO error".to_string())
        }
    }
}

pub fn uu_app<'a>() -> App<'a> {
    App::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .setting(AppSettings::InferLongArgs)
        .arg(
            Arg::new(options::INFILE)
                .long(options::INFILE)
                .overrides_with(options::INFILE)
                .takes_value(true)
                .require_equals(true)
                .value_name("FILE")
                .help("(alternatively if=FILE) specifies the file used for input. When not specified, stdin is used instead")
        )
        .arg(
            Arg::new(options::OUTFILE)
                .long(options::OUTFILE)
                .overrides_with(options::OUTFILE)
                .takes_value(true)
                .require_equals(true)
                .value_name("FILE")
                .help("(alternatively of=FILE) specifies the file used for output. When not specified, stdout is used instead")
        )
        .arg(
            Arg::new(options::IBS)
                .long(options::IBS)
                .overrides_with(options::IBS)
                .takes_value(true)
                .require_equals(true)
                .value_name("N")
                .help("(alternatively ibs=N) specifies the size of buffer used for reads (default: 512). Multiplier strings permitted.")
        )
        .arg(
            Arg::new(options::OBS)
                .long(options::OBS)
                .overrides_with(options::OBS)
                .takes_value(true)
                .require_equals(true)
                .value_name("N")
                .help("(alternatively obs=N) specifies the size of buffer used for writes (default: 512). Multiplier strings permitted.")
        )
        .arg(
            Arg::new(options::BS)
                .long(options::BS)
                .overrides_with(options::BS)
                .takes_value(true)
                .require_equals(true)
                .value_name("N")
                .help("(alternatively bs=N) specifies ibs=N and obs=N (default: 512). If ibs or obs are also specified, bs=N takes precedence. Multiplier strings permitted.")
        )
        .arg(
            Arg::new(options::CBS)
                .long(options::CBS)
                .overrides_with(options::CBS)
                .takes_value(true)
                .require_equals(true)
                .value_name("N")
                .help("(alternatively cbs=BYTES) specifies the 'conversion block size' in bytes. Applies to the conv=block, and conv=unblock operations. Multiplier strings permitted.")
        )
        .arg(
            Arg::new(options::SKIP)
                .long(options::SKIP)
                .overrides_with(options::SKIP)
                .takes_value(true)
                .require_equals(true)
                .value_name("N")
                .help("(alternatively skip=N) causes N ibs-sized records of input to be skipped before beginning copy/convert operations. See iflag=count_bytes if skipping N bytes is preferred. Multiplier strings permitted.")
        )
        .arg(
            Arg::new(options::SEEK)
                .long(options::SEEK)
                .overrides_with(options::SEEK)
                .takes_value(true)
                .require_equals(true)
                .value_name("N")
                .help("(alternatively seek=N) seeks N obs-sized records into output before beginning copy/convert operations. See oflag=seek_bytes if seeking N bytes is preferred. Multiplier strings permitted.")
        )
        .arg(
            Arg::new(options::COUNT)
                .long(options::COUNT)
                .overrides_with(options::COUNT)
                .takes_value(true)
                .require_equals(true)
                .value_name("N")
                .help("(alternatively count=N) stop reading input after N ibs-sized read operations rather than proceeding until EOF. See iflag=count_bytes if stopping after N bytes is preferred. Multiplier strings permitted.")
        )
        .arg(
            Arg::new(options::STATUS)
                .long(options::STATUS)
                .overrides_with(options::STATUS)
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
            Arg::new(options::CONV)
                .long(options::CONV)
                .takes_value(true)
                .multiple_occurrences(true)
                .use_delimiter(true)
                .require_delimiter(true)
                .multiple_values(true)
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
            Arg::new(options::IFLAG)
                .long(options::IFLAG)
                .takes_value(true)
                .multiple_occurrences(true)
                .use_delimiter(true)
                .require_delimiter(true)
                .multiple_values(true)
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
            Arg::new(options::OFLAG)
                .long(options::OFLAG)
                .takes_value(true)
                .multiple_occurrences(true)
                .use_delimiter(true)
                .require_delimiter(true)
                .multiple_values(true)
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

#[cfg(test)]
mod tests {

    use crate::datastructures::{IConvFlags, IFlags, OConvFlags};
    use crate::ReadStat;
    use crate::{block, calc_bsize, unblock, uu_app, Input, Output, OutputTrait};

    use std::cmp;
    use std::fs;
    use std::fs::File;
    use std::io;
    use std::io::{BufReader, Read};

    struct LazyReader<R: Read> {
        src: R,
    }

    impl<R: Read> Read for LazyReader<R> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let reduced = cmp::max(buf.len() / 2, 1);
            self.src.read(&mut buf[..reduced])
        }
    }

    const NEWLINE: u8 = b'\n';
    const SPACE: u8 = b' ';

    #[test]
    fn block_test_no_nl() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8];
        let res = block(&buf, 4, &mut rs);

        assert_eq!(res, vec![vec![0u8, 1u8, 2u8, 3u8],]);
    }

    #[test]
    fn block_test_no_nl_short_record() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8];
        let res = block(&buf, 8, &mut rs);

        assert_eq!(
            res,
            vec![vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],]
        );
    }

    #[test]
    fn block_test_no_nl_trunc() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8, 4u8];
        let res = block(&buf, 4, &mut rs);

        // Commented section(s) should be truncated and appear for reference only.
        assert_eq!(res, vec![vec![0u8, 1u8, 2u8, 3u8 /*, 4u8*/],]);
        assert_eq!(rs.records_truncated, 1);
    }

    #[test]
    fn block_test_nl_gt_cbs_trunc() {
        let mut rs = ReadStat::default();
        let buf = [
            0u8, 1u8, 2u8, 3u8, 4u8, NEWLINE, 0u8, 1u8, 2u8, 3u8, 4u8, NEWLINE, 5u8, 6u8, 7u8, 8u8,
        ];
        let res = block(&buf, 4, &mut rs);

        assert_eq!(
            res,
            vec![
                // Commented section(s) should be truncated and appear for reference only.
                vec![0u8, 1u8, 2u8, 3u8],
                // vec![4u8, SPACE, SPACE, SPACE],
                vec![0u8, 1u8, 2u8, 3u8],
                // vec![4u8, SPACE, SPACE, SPACE],
                vec![5u8, 6u8, 7u8, 8u8],
            ]
        );
        assert_eq!(rs.records_truncated, 2);
    }

    #[test]
    fn block_test_surrounded_nl() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8, NEWLINE, 4u8, 5u8, 6u8, 7u8, 8u8];
        let res = block(&buf, 8, &mut rs);

        assert_eq!(
            res,
            vec![
                vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
                vec![4u8, 5u8, 6u8, 7u8, 8u8, SPACE, SPACE, SPACE],
            ]
        );
    }

    #[test]
    fn block_test_multiple_nl_same_cbs_block() {
        let mut rs = ReadStat::default();
        let buf = [
            0u8, 1u8, 2u8, 3u8, NEWLINE, 4u8, NEWLINE, 5u8, 6u8, 7u8, 8u8, 9u8,
        ];
        let res = block(&buf, 8, &mut rs);

        assert_eq!(
            res,
            vec![
                vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
                vec![4u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
                vec![5u8, 6u8, 7u8, 8u8, 9u8, SPACE, SPACE, SPACE],
            ]
        );
    }

    #[test]
    fn block_test_multiple_nl_diff_cbs_block() {
        let mut rs = ReadStat::default();
        let buf = [
            0u8, 1u8, 2u8, 3u8, NEWLINE, 4u8, 5u8, 6u8, 7u8, NEWLINE, 8u8, 9u8,
        ];
        let res = block(&buf, 8, &mut rs);

        assert_eq!(
            res,
            vec![
                vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
                vec![4u8, 5u8, 6u8, 7u8, SPACE, SPACE, SPACE, SPACE],
                vec![8u8, 9u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
            ]
        );
    }

    #[test]
    fn block_test_end_nl_diff_cbs_block() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8, NEWLINE];
        let res = block(&buf, 4, &mut rs);

        assert_eq!(res, vec![vec![0u8, 1u8, 2u8, 3u8],]);
    }

    #[test]
    fn block_test_end_nl_same_cbs_block() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, NEWLINE];
        let res = block(&buf, 4, &mut rs);

        assert_eq!(res, vec![vec![0u8, 1u8, 2u8, SPACE]]);
    }

    #[test]
    fn block_test_double_end_nl() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, NEWLINE, NEWLINE];
        let res = block(&buf, 4, &mut rs);

        assert_eq!(
            res,
            vec![vec![0u8, 1u8, 2u8, SPACE], vec![SPACE, SPACE, SPACE, SPACE],]
        );
    }

    #[test]
    fn block_test_start_nl() {
        let mut rs = ReadStat::default();
        let buf = [NEWLINE, 0u8, 1u8, 2u8, 3u8];
        let res = block(&buf, 4, &mut rs);

        assert_eq!(
            res,
            vec![vec![SPACE, SPACE, SPACE, SPACE], vec![0u8, 1u8, 2u8, 3u8],]
        );
    }

    #[test]
    fn block_test_double_surrounded_nl_no_trunc() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8, NEWLINE, NEWLINE, 4u8, 5u8, 6u8, 7u8];
        let res = block(&buf, 8, &mut rs);

        assert_eq!(
            res,
            vec![
                vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
                vec![SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
                vec![4u8, 5u8, 6u8, 7u8, SPACE, SPACE, SPACE, SPACE],
            ]
        );
    }

    #[test]
    fn block_test_double_surrounded_nl_double_trunc() {
        let mut rs = ReadStat::default();
        let buf = [
            0u8, 1u8, 2u8, 3u8, NEWLINE, NEWLINE, 4u8, 5u8, 6u8, 7u8, 8u8,
        ];
        let res = block(&buf, 4, &mut rs);

        assert_eq!(
            res,
            vec![
                // Commented section(s) should be truncated and appear for reference only.
                vec![0u8, 1u8, 2u8, 3u8],
                vec![SPACE, SPACE, SPACE, SPACE],
                vec![4u8, 5u8, 6u8, 7u8 /*, 8u8*/],
            ]
        );
        assert_eq!(rs.records_truncated, 1);
    }

    #[test]
    fn unblock_test_full_cbs() {
        let buf = [0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8];
        let res = unblock(&buf, 8);

        assert_eq!(res, vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, NEWLINE],);
    }

    #[test]
    fn unblock_test_all_space() {
        let buf = [SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE];
        let res = unblock(&buf, 8);

        assert_eq!(res, vec![NEWLINE],);
    }

    #[test]
    fn unblock_test_decoy_spaces() {
        let buf = [0u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, 7u8];
        let res = unblock(&buf, 8);

        assert_eq!(
            res,
            vec![0u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, 7u8, NEWLINE],
        );
    }

    #[test]
    fn unblock_test_strip_single_cbs() {
        let buf = [0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE];
        let res = unblock(&buf, 8);

        assert_eq!(res, vec![0u8, 1u8, 2u8, 3u8, NEWLINE],);
    }

    #[test]
    fn unblock_test_strip_multi_cbs() {
        let buf = vec![
            vec![0u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
            vec![0u8, 1u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
            vec![0u8, 1u8, 2u8, SPACE, SPACE, SPACE, SPACE, SPACE],
            vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        let res = unblock(&buf, 8);

        let exp = vec![
            vec![0u8, NEWLINE],
            vec![0u8, 1u8, NEWLINE],
            vec![0u8, 1u8, 2u8, NEWLINE],
            vec![0u8, 1u8, 2u8, 3u8, NEWLINE],
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        assert_eq!(res, exp);
    }

    #[test]
    fn bsize_test_primes() {
        let (n, m) = (7901, 7919);
        let res = calc_bsize(n, m);
        assert!(res % n == 0);
        assert!(res % m == 0);

        assert_eq!(res, n * m);
    }

    #[test]
    fn bsize_test_rel_prime_obs_greater() {
        let (n, m) = (7 * 5119, 13 * 5119);
        let res = calc_bsize(n, m);
        assert!(res % n == 0);
        assert!(res % m == 0);

        assert_eq!(res, 7 * 13 * 5119);
    }

    #[test]
    fn bsize_test_rel_prime_ibs_greater() {
        let (n, m) = (13 * 5119, 7 * 5119);
        let res = calc_bsize(n, m);
        assert!(res % n == 0);
        assert!(res % m == 0);

        assert_eq!(res, 7 * 13 * 5119);
    }

    #[test]
    fn bsize_test_3fac_rel_prime() {
        let (n, m) = (11 * 13 * 5119, 7 * 11 * 5119);
        let res = calc_bsize(n, m);
        assert!(res % n == 0);
        assert!(res % m == 0);

        assert_eq!(res, 7 * 11 * 13 * 5119);
    }

    #[test]
    fn bsize_test_ibs_greater() {
        let (n, m) = (512 * 1024, 256 * 1024);
        let res = calc_bsize(n, m);
        assert!(res % n == 0);
        assert!(res % m == 0);

        assert_eq!(res, n);
    }

    #[test]
    fn bsize_test_obs_greater() {
        let (n, m) = (256 * 1024, 512 * 1024);
        let res = calc_bsize(n, m);
        assert!(res % n == 0);
        assert!(res % m == 0);

        assert_eq!(res, m);
    }

    #[test]
    fn bsize_test_bs_eq() {
        let (n, m) = (1024, 1024);
        let res = calc_bsize(n, m);
        assert!(res % n == 0);
        assert!(res % m == 0);

        assert_eq!(res, m);
    }

    #[test]
    #[should_panic]
    fn test_nocreat_causes_failure_when_ofile_doesnt_exist() {
        let args = vec![
            String::from("dd"),
            String::from("--conv=nocreat"),
            String::from("--of=not-a-real.file"),
        ];

        let matches = uu_app().try_get_matches_from(args).unwrap();
        let _ = Output::<File>::new(&matches).unwrap();
    }

    #[test]
    fn test_deadbeef_16_delayed() {
        let input = Input {
            src: LazyReader {
                src: File::open("./test-resources/deadbeef-16.test").unwrap(),
            },
            non_ascii: false,
            ibs: 16,
            print_level: None,
            count: None,
            cflags: IConvFlags {
                sync: Some(0),
                ..IConvFlags::default()
            },
            iflags: IFlags::default(),
        };

        let output = Output {
            dst: File::create("./test-resources/FAILED-deadbeef-16-delayed.test").unwrap(),
            obs: 32,
            cflags: OConvFlags::default(),
        };

        output.dd_out(input).unwrap();

        let tmp_fname = "./test-resources/FAILED-deadbeef-16-delayed.test";
        let spec = File::open("./test-resources/deadbeef-16.spec").unwrap();

        let res = File::open(tmp_fname).unwrap();
        // Check test file isn't empty (unless spec file is too)
        assert_eq!(
            res.metadata().unwrap().len(),
            spec.metadata().unwrap().len()
        );

        let spec = BufReader::new(spec);
        let res = BufReader::new(res);

        // Check all bytes match
        for (b_res, b_spec) in res.bytes().zip(spec.bytes()) {
            assert_eq!(b_res.unwrap(), b_spec.unwrap());
        }

        fs::remove_file(tmp_fname).unwrap();
    }

    #[test]
    fn test_random_73k_test_lazy_fullblock() {
        let input = Input {
            src: LazyReader {
                src: File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test")
                    .unwrap(),
            },
            non_ascii: false,
            ibs: 521,
            print_level: None,
            count: None,
            cflags: IConvFlags::default(),
            iflags: IFlags {
                fullblock: true,
                ..IFlags::default()
            },
        };

        let output = Output {
            dst: File::create("./test-resources/FAILED-random_73k_test_lazy_fullblock.test")
                .unwrap(),
            obs: 1031,
            cflags: OConvFlags::default(),
        };

        output.dd_out(input).unwrap();

        let tmp_fname = "./test-resources/FAILED-random_73k_test_lazy_fullblock.test";
        let spec =
            File::open("./test-resources/random-5828891cb1230748e146f34223bbd3b5.test").unwrap();

        let res = File::open(tmp_fname).unwrap();
        // Check test file isn't empty (unless spec file is too)
        assert_eq!(
            res.metadata().unwrap().len(),
            spec.metadata().unwrap().len()
        );

        let spec = BufReader::new(spec);
        let res = BufReader::new(res);

        // Check all bytes match
        for (b_res, b_spec) in res.bytes().zip(spec.bytes()) {
            assert_eq!(b_res.unwrap(), b_spec.unwrap());
        }

        fs::remove_file(tmp_fname).unwrap();
    }
}
