// This file is part of the uutils coreutils package.
//
// (c) Tyler Steele <tyler.steele@protonmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore fname, tname, fpath, specfile, testfile, unspec, ifile, ofile, outfile, fullblock, urand, fileio, atoe, atoibm, behaviour, bmax, bremain, cflags, creat, ctable, ctty, datastructures, doesnt, etoa, fileout, fname, gnudd, iconvflags, nocache, noctty, noerror, nofollow, nolinks, nonblock, oconvflags, outfile, parseargs, rlen, rmax, rremain, rsofar, rstat, sigusr, wlen, wstat seekable

mod datastructures;
use datastructures::*;

mod parseargs;
use parseargs::Matches;

mod conversion_tables;
use conversion_tables::*;

mod progress;
use progress::{gen_prog_updater, ProgUpdate, ReadStat, StatusLevel, WriteStat};

mod blocks;
use blocks::conv_block_unblock_helper;

use std::cmp;
use std::convert::TryInto;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, Write};
#[cfg(target_os = "linux")]
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time;

use clap::{crate_version, Arg, ArgMatches, Command};
use gcd::Gcd;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult};
use uucore::{show_error, InvalidEncodingHandling};

const ABOUT: &str = "copy, and optionally convert, a file system resource";
const BUF_INIT_BYTE: u8 = 0xDD;

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
            if let Err(e) = i.read_skip(amt) {
                if let io::ErrorKind::UnexpectedEof = e.kind() {
                    show_error!("'standard input': cannot skip to specified offset");
                } else {
                    return io::Result::Err(e)
                        .map_err_context(|| "I/O error while skipping".to_string());
                }
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

    /// Skips amount_to_read bytes from the Input by copying into a sink
    fn read_skip(&mut self, amount_to_read: u64) -> std::io::Result<()> {
        let copy_result = io::copy(&mut self.src.by_ref().take(amount_to_read), &mut io::sink());
        if let Ok(n) = copy_result {
            if n != amount_to_read {
                io::Result::Err(io::Error::new(io::ErrorKind::UnexpectedEof, ""))
            } else {
                Ok(())
            }
        } else {
            io::Result::Err(copy_result.unwrap_err())
        }
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
            io::copy(&mut io::repeat(0u8).take(amt as u64), &mut dst)
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
            Some(StatusLevel::Noxfer) => prog_update.print_io_lines(),
            Some(StatusLevel::Progress) | None => prog_update.print_transfer_stats(),
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

            // Seek to the index in the output file, truncating if requested.
            //
            // Calling `set_len()` may result in an error (for
            // example, when calling it on `/dev/null`), but we don't
            // want to terminate the process when that happens.
            // Instead, we suppress the error by calling
            // `Result::ok()`. This matches the behavior of GNU `dd`
            // when given the command-line argument `of=/dev/null`.
            let i = seek.unwrap_or(0);
            if !cflags.notrunc {
                dst.set_len(i).ok();
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
            let rsofar = rstat.reads_complete + rstat.reads_partial;
            let rremain = rmax - rsofar;
            cmp::min(ideal_bsize as u64, rremain * ibs as u64) as usize
        }
        Some(CountType::Bytes(bmax)) => {
            let bmax: u128 = (*bmax).try_into().unwrap();
            let bremain: u128 = bmax - wstat.bytes_total;
            cmp::min(ideal_bsize as u128, bremain as u128) as usize
        }
        None => ideal_bsize,
    }
}

// Decide if the current progress is below a count=N limit or return
// true if no such limit is set.
fn below_count_limit(count: &Option<CountType>, rstat: &ReadStat, wstat: &WriteStat) -> bool {
    match count {
        Some(CountType::Reads(n)) => {
            let n = *n;
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

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .infer_long_args(true)
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
                .use_value_delimiter(true)
                .require_value_delimiter(true)
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
                .use_value_delimiter(true)
                .require_value_delimiter(true)
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
                .use_value_delimiter(true)
                .require_value_delimiter(true)
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
    use crate::{calc_bsize, uu_app, Input, Output, OutputTrait};

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
