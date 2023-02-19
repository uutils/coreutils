// This file is part of the uutils coreutils package.
//
// (c) Tyler Steele <tyler.steele@protonmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore fname, tname, fpath, specfile, testfile, unspec, ifile, ofile, outfile, fullblock, urand, fileio, atoe, atoibm, behaviour, bmax, bremain, cflags, creat, ctable, ctty, datastructures, doesnt, etoa, fileout, fname, gnudd, iconvflags, iseek, nocache, noctty, noerror, nofollow, nolinks, nonblock, oconvflags, oseek, outfile, parseargs, rlen, rmax, rremain, rsofar, rstat, sigusr, wlen, wstat seekable oconv canonicalized

mod datastructures;
use datastructures::*;

mod parseargs;
use parseargs::Parser;

mod conversion_tables;

mod progress;
use progress::{gen_prog_updater, ProgUpdate, ReadStat, StatusLevel, WriteStat};

mod blocks;
use blocks::conv_block_unblock_helper;

mod numbers;

use std::cmp;
use std::env;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Stdout, Write};
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time;

use clap::{crate_version, Arg, Command};
use gcd::Gcd;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult};
use uucore::{format_usage, help_about, help_section, help_usage, show_error};

const ABOUT: &str = help_about!("dd.md");
const AFTER_HELP: &str = help_section!("after help", "dd.md");
const USAGE: &str = help_usage!("dd.md");
const BUF_INIT_BYTE: u8 = 0xDD;

/// Final settings after parsing
#[derive(Default)]
struct Settings {
    infile: Option<String>,
    outfile: Option<String>,
    ibs: usize,
    obs: usize,
    skip: u64,
    seek: u64,
    count: Option<Num>,
    iconv: IConvFlags,
    iflags: IFlags,
    oconv: OConvFlags,
    oflags: OFlags,
    status: Option<StatusLevel>,
}

/// A number in blocks or bytes
///
/// Some values (seek, skip, iseek, oseek) can have values either in blocks or in bytes.
/// We need to remember this because the size of the blocks (ibs) is only known after parsing
/// all the arguments.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Num {
    Blocks(u64),
    Bytes(u64),
}

impl Num {
    fn force_bytes_if(self, force: bool) -> Self {
        match self {
            Self::Blocks(n) if force => Self::Bytes(n),
            count => count,
        }
    }

    fn to_bytes(self, block_size: u64) -> u64 {
        match self {
            Self::Blocks(n) => n * block_size,
            Self::Bytes(n) => n,
        }
    }
}

struct Input<'a, R: Read> {
    src: R,
    settings: &'a Settings,
}

impl<'a> Input<'a, io::Stdin> {
    fn new(settings: &'a Settings) -> UResult<Self> {
        let mut input = Self {
            src: io::stdin(),
            settings,
        };

        if settings.skip > 0 {
            if let Err(e) = input.read_skip(settings.skip) {
                if let io::ErrorKind::UnexpectedEof = e.kind() {
                    show_error!("'standard input': cannot skip to specified offset");
                } else {
                    return io::Result::Err(e)
                        .map_err_context(|| "I/O error while skipping".to_string());
                }
            }
        }

        Ok(input)
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
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

impl<'a> Input<'a, File> {
    fn new(filename: &Path, settings: &'a Settings) -> UResult<Self> {
        let mut src = {
            let mut opts = OpenOptions::new();
            opts.read(true);

            #[cfg(any(target_os = "linux", target_os = "android"))]
            if let Some(libc_flags) = make_linux_iflags(&settings.iflags) {
                opts.custom_flags(libc_flags);
            }

            opts.open(filename)
                .map_err_context(|| format!("failed to open {}", filename.quote()))?
        };

        if settings.skip > 0 {
            src.seek(io::SeekFrom::Start(settings.skip))
                .map_err_context(|| "failed to seek in input file".to_string())?;
        }

        Ok(Self { src, settings })
    }
}

impl<'a, R: Read> Read for Input<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut base_idx = 0;
        let target_len = buf.len();
        loop {
            match self.src.read(&mut buf[base_idx..]) {
                Ok(0) => return Ok(base_idx),
                Ok(rlen) if self.settings.iflags.fullblock => {
                    base_idx += rlen;

                    if base_idx >= target_len {
                        return Ok(target_len);
                    }
                }
                Ok(len) => return Ok(len),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) if self.settings.iconv.noerror => return Ok(base_idx),
                Err(e) => return Err(e),
            }
        }
    }
}

impl<'a, R: Read> Input<'a, R> {
    /// Fills a given buffer.
    /// Reads in increments of 'self.ibs'.
    /// The start of each ibs-sized read follows the previous one.
    fn fill_consecutive(&mut self, buf: &mut Vec<u8>) -> std::io::Result<ReadStat> {
        let mut reads_complete = 0;
        let mut reads_partial = 0;
        let mut bytes_total = 0;

        for chunk in buf.chunks_mut(self.settings.ibs) {
            match self.read(chunk)? {
                rlen if rlen == self.settings.ibs => {
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
            let next_blk = cmp::min(base_idx + self.settings.ibs, buf.len());
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

            base_idx += self.settings.ibs;
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

enum Density {
    Sparse,
    Dense,
}

/// Data destinations.
enum Dest {
    /// Output to stdout.
    Stdout(Stdout),

    /// Output to a file.
    ///
    /// The [`Density`] component indicates whether to attempt to
    /// write a sparse file when all-zero blocks are encountered.
    File(File, Density),
}

impl Dest {
    fn fsync(&mut self) -> io::Result<()> {
        match self {
            Self::Stdout(stdout) => stdout.flush(),
            Self::File(f, _) => {
                f.flush()?;
                f.sync_all()
            }
        }
    }

    fn fdatasync(&mut self) -> io::Result<()> {
        match self {
            Self::Stdout(stdout) => stdout.flush(),
            Self::File(f, _) => {
                f.flush()?;
                f.sync_data()
            }
        }
    }

    fn seek(&mut self, n: u64) -> io::Result<u64> {
        match self {
            Self::Stdout(stdout) => io::copy(&mut io::repeat(0).take(n), stdout),
            Self::File(f, _) => f.seek(io::SeekFrom::Start(n)),
        }
    }

    /// Truncate the underlying file to the current stream position, if possible.
    fn truncate(&mut self) -> io::Result<()> {
        match self {
            Self::Stdout(_) => Ok(()),
            Self::File(f, _) => {
                let pos = f.stream_position()?;
                f.set_len(pos)
            }
        }
    }
}

/// Decide whether the given buffer is all zeros.
fn is_sparse(buf: &[u8]) -> bool {
    buf.iter().all(|&e| e == 0u8)
}

impl Write for Dest {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::File(f, Density::Sparse) if is_sparse(buf) => {
                let seek_amt: i64 = buf
                    .len()
                    .try_into()
                    .expect("Internal dd Error: Seek amount greater than signed 64-bit integer");
                f.seek(io::SeekFrom::Current(seek_amt))?;
                Ok(buf.len())
            }
            Self::File(f, _) => f.write(buf),
            Self::Stdout(stdout) => stdout.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Stdout(stdout) => stdout.flush(),
            Self::File(f, _) => f.flush(),
        }
    }
}

/// The destination of the data, configured with the given settings.
///
/// Use the [`Output::new_stdout`] or [`Output::new_file`] functions
/// to construct a new instance of this struct. Then use the
/// [`Output::dd_out`] function to execute the main copy operation for
/// `dd`.
struct Output<'a> {
    /// The destination to which bytes will be written.
    dst: Dest,

    /// Configuration settings for how to read and write the data.
    settings: &'a Settings,
}

impl<'a> Output<'a> {
    /// Instantiate this struct with stdout as a destination.
    fn new_stdout(settings: &'a Settings) -> UResult<Self> {
        let mut dst = Dest::Stdout(io::stdout());
        dst.seek(settings.seek)
            .map_err_context(|| "write error".to_string())?;
        Ok(Self { dst, settings })
    }

    /// Instantiate this struct with the named file as a destination.
    fn new_file(filename: &Path, settings: &'a Settings) -> UResult<Self> {
        fn open_dst(path: &Path, cflags: &OConvFlags, oflags: &OFlags) -> Result<File, io::Error> {
            let mut opts = OpenOptions::new();
            opts.write(true)
                .create(!cflags.nocreat)
                .create_new(cflags.excl)
                .append(oflags.append);

            #[cfg(any(target_os = "linux", target_os = "android"))]
            if let Some(libc_flags) = make_linux_oflags(oflags) {
                opts.custom_flags(libc_flags);
            }

            opts.open(path)
        }

        let dst = open_dst(filename, &settings.oconv, &settings.oflags)
            .map_err_context(|| format!("failed to open {}", filename.quote()))?;

        // Seek to the index in the output file, truncating if requested.
        //
        // Calling `set_len()` may result in an error (for example,
        // when calling it on `/dev/null`), but we don't want to
        // terminate the process when that happens.  Instead, we
        // suppress the error by calling `Result::ok()`. This matches
        // the behavior of GNU `dd` when given the command-line
        // argument `of=/dev/null`.
        if !settings.oconv.notrunc {
            dst.set_len(settings.seek).ok();
        }
        let density = if settings.oconv.sparse {
            Density::Sparse
        } else {
            Density::Dense
        };
        let mut dst = Dest::File(dst, density);
        dst.seek(settings.seek)
            .map_err_context(|| "failed to seek in output file".to_string())?;
        Ok(Self { dst, settings })
    }

    /// Write the given bytes one block at a time.
    ///
    /// This may write partial blocks (for example, if the underlying
    /// call to [`Write::write`] writes fewer than `buf.len()`
    /// bytes). The returned [`WriteStat`] object will include the
    /// number of partial and complete blocks written during execution
    /// of this function.
    fn write_blocks(&mut self, buf: &[u8]) -> io::Result<WriteStat> {
        let mut writes_complete = 0;
        let mut writes_partial = 0;
        let mut bytes_total = 0;

        for chunk in buf.chunks(self.settings.obs) {
            let wlen = self.dst.write(chunk)?;
            if wlen < self.settings.obs {
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

    /// Flush the output to disk, if configured to do so.
    fn sync(&mut self) -> std::io::Result<()> {
        if self.settings.oconv.fsync {
            self.dst.fsync()
        } else if self.settings.oconv.fdatasync {
            self.dst.fdatasync()
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
        let mut rstat = ReadStat::default();
        let mut wstat = WriteStat::default();

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
        let bsize = calc_bsize(i.settings.ibs, self.settings.obs);

        // Start a thread that reports transfer progress.
        //
        // The `dd` program reports its progress after every block is written,
        // at most every 1 second, and only if `status=progress` is given on
        // the command-line or a SIGUSR1 signal is received. We
        // perform this reporting in a new thread so as not to take
        // any CPU time away from the actual reading and writing of
        // data. We send a `ProgUpdate` from the transmitter `prog_tx`
        // to the receives `rx`, and the receiver prints the transfer
        // information.
        let (prog_tx, rx) = mpsc::channel();
        let output_thread = thread::spawn(gen_prog_updater(rx, i.settings.status));
        let mut progress_as_secs = 0;

        // Optimization: if no blocks are to be written, then don't
        // bother allocating any buffers.
        if let Some(Num::Blocks(0) | Num::Bytes(0)) = i.settings.count {
            return self.finalize(rstat, wstat, start, &prog_tx, output_thread);
        };

        // Create a common buffer with a capacity of the block size.
        // This is the max size needed.
        let mut buf = vec![BUF_INIT_BYTE; bsize];

        // The main read/write loop.
        //
        // Each iteration reads blocks from the input and writes
        // blocks to this output. Read/write statistics are updated on
        // each iteration and cumulative statistics are reported to
        // the progress reporting thread.
        while below_count_limit(&i.settings.count, &rstat, &wstat) {
            // Read a block from the input then write the block to the output.
            //
            // As an optimization, make an educated guess about the
            // best buffer size for reading based on the number of
            // blocks already read and the number of blocks remaining.
            let loop_bsize =
                calc_loop_bsize(&i.settings.count, &rstat, &wstat, i.settings.ibs, bsize);
            let rstat_update = read_helper(&mut i, &mut buf, loop_bsize)?;
            if rstat_update.is_empty() {
                break;
            }
            let wstat_update = self.write_blocks(&buf)?;

            // Update the read/write stats and inform the progress thread once per second.
            //
            // If the receiver is disconnected, `send()` returns an
            // error. Since it is just reporting progress and is not
            // crucial to the operation of `dd`, let's just ignore the
            // error.
            rstat += rstat_update;
            wstat += wstat_update;
            let prog_update = ProgUpdate::new(rstat, wstat, start.elapsed(), false);
            if prog_update.duration.as_secs() >= progress_as_secs {
                progress_as_secs = prog_update.duration.as_secs() + 1;
                prog_tx.send(prog_update).unwrap_or(());
            }
        }
        self.finalize(rstat, wstat, start, &prog_tx, output_thread)
    }

    /// Flush output, print final stats, and join with the progress thread.
    fn finalize<T>(
        &mut self,
        rstat: ReadStat,
        wstat: WriteStat,
        start: time::Instant,
        prog_tx: &mpsc::Sender<ProgUpdate>,
        output_thread: thread::JoinHandle<T>,
    ) -> std::io::Result<()> {
        // Flush the output, if configured to do so.
        self.sync()?;

        // Truncate the file to the final cursor location.
        //
        // Calling `set_len()` may result in an error (for example,
        // when calling it on `/dev/null`), but we don't want to
        // terminate the process when that happens. Instead, we
        // suppress the error by calling `Result::ok()`. This matches
        // the behavior of GNU `dd` when given the command-line
        // argument `of=/dev/null`.
        if !self.settings.oconv.notrunc {
            self.dst.truncate().ok();
        }

        // Print the final read/write statistics.
        let prog_update = ProgUpdate::new(rstat, wstat, start.elapsed(), true);
        prog_tx.send(prog_update).unwrap_or(());
        // Wait for the output thread to finish
        output_thread
            .join()
            .expect("Failed to join with the output thread.");
        Ok(())
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
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

/// Read helper performs read operations common to all dd reads, and dispatches the buffer to relevant helper functions as dictated by the operations requested by the user.
fn read_helper<R: Read>(
    i: &mut Input<R>,
    buf: &mut Vec<u8>,
    bsize: usize,
) -> std::io::Result<ReadStat> {
    // Local Helper Fns -------------------------------------------------
    fn perform_swab(buf: &mut [u8]) {
        for base in (1..buf.len()).step_by(2) {
            buf.swap(base, base - 1);
        }
    }
    // ------------------------------------------------------------------
    // Read
    // Resize the buffer to the bsize. Any garbage data in the buffer is overwritten or truncated, so there is no need to fill with BUF_INIT_BYTE first.
    buf.resize(bsize, BUF_INIT_BYTE);

    let mut rstat = match i.settings.iconv.sync {
        Some(ch) => i.fill_blocks(buf, ch)?,
        _ => i.fill_consecutive(buf)?,
    };
    // Return early if no data
    if rstat.reads_complete == 0 && rstat.reads_partial == 0 {
        return Ok(rstat);
    }

    // Perform any conv=x[,x...] options
    if i.settings.iconv.swab {
        perform_swab(buf);
    }

    match i.settings.iconv.mode {
        Some(ref mode) => {
            *buf = conv_block_unblock_helper(buf.clone(), mode, &mut rstat);
            Ok(rstat)
        }
        None => Ok(rstat),
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
    count: &Option<Num>,
    rstat: &ReadStat,
    wstat: &WriteStat,
    ibs: usize,
    ideal_bsize: usize,
) -> usize {
    match count {
        Some(Num::Blocks(rmax)) => {
            let rsofar = rstat.reads_complete + rstat.reads_partial;
            let rremain = rmax - rsofar;
            cmp::min(ideal_bsize as u64, rremain * ibs as u64) as usize
        }
        Some(Num::Bytes(bmax)) => {
            let bmax: u128 = (*bmax).try_into().unwrap();
            let bremain: u128 = bmax - wstat.bytes_total;
            cmp::min(ideal_bsize as u128, bremain) as usize
        }
        None => ideal_bsize,
    }
}

// Decide if the current progress is below a count=N limit or return
// true if no such limit is set.
fn below_count_limit(count: &Option<Num>, rstat: &ReadStat, wstat: &WriteStat) -> bool {
    match count {
        Some(Num::Blocks(n)) => {
            let n = *n;
            rstat.reads_complete + rstat.reads_partial <= n
        }
        Some(Num::Bytes(n)) => {
            let n = (*n).try_into().unwrap();
            wstat.bytes_total <= n
        }
        None => true,
    }
}

/// Canonicalized file name of `/dev/stdout`.
///
/// For example, if this process were invoked from the command line as
/// `dd`, then this function returns the [`OsString`] form of
/// `"/dev/stdout"`. However, if this process were invoked as `dd >
/// outfile`, then this function returns the canonicalized path to
/// `outfile`, something like `"/path/to/outfile"`.
fn stdout_canonicalized() -> OsString {
    match Path::new("/dev/stdout").canonicalize() {
        Ok(p) => p.into_os_string(),
        Err(_) => OsString::from("/dev/stdout"),
    }
}

/// Decide whether stdout is being redirected to a seekable file.
///
/// For example, if this process were invoked from the command line as
///
/// ```sh
/// dd if=/dev/zero bs=1 count=10 seek=5 > /dev/sda1
/// ```
///
/// where `/dev/sda1` is a seekable block device then this function
/// would return true. If invoked as
///
/// ```sh
/// dd if=/dev/zero bs=1 count=10 seek=5
/// ```
///
/// then this function would return false.
fn is_stdout_redirected_to_seekable_file() -> bool {
    let s = stdout_canonicalized();
    let p = Path::new(&s);
    match File::open(p) {
        Ok(mut f) => {
            f.stream_position().is_ok() && f.seek(SeekFrom::End(0)).is_ok() && f.rewind().is_ok()
        }
        Err(_) => false,
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_ignore();

    let matches = uu_app().try_get_matches_from(args)?;

    let settings: Settings = Parser::new().parse(
        &matches
            .get_many::<String>(options::OPERANDS)
            .unwrap_or_default()
            .map(|s| s.as_ref())
            .collect::<Vec<_>>()[..],
    )?;

    match (&settings.infile, &settings.outfile) {
        (Some(infile), Some(outfile)) => {
            let i = Input::<File>::new(Path::new(&infile), &settings)?;
            let o = Output::new_file(Path::new(&outfile), &settings)?;
            o.dd_out(i).map_err_context(|| "IO error".to_string())
        }
        (None, Some(outfile)) => {
            let i = Input::<io::Stdin>::new(&settings)?;
            let o = Output::new_file(Path::new(&outfile), &settings)?;
            o.dd_out(i).map_err_context(|| "IO error".to_string())
        }
        (Some(infile), None) => {
            let i = Input::<File>::new(Path::new(&infile), &settings)?;
            if is_stdout_redirected_to_seekable_file() {
                let filename = stdout_canonicalized();
                let o = Output::new_file(Path::new(&filename), &settings)?;
                o.dd_out(i).map_err_context(|| "IO error".to_string())
            } else {
                let o = Output::new_stdout(&settings)?;
                o.dd_out(i).map_err_context(|| "IO error".to_string())
            }
        }
        (None, None) => {
            let i = Input::<io::Stdin>::new(&settings)?;
            if is_stdout_redirected_to_seekable_file() {
                let filename = stdout_canonicalized();
                let o = Output::new_file(Path::new(&filename), &settings)?;
                o.dd_out(i).map_err_context(|| "IO error".to_string())
            } else {
                let o = Output::new_stdout(&settings)?;
                o.dd_out(i).map_err_context(|| "IO error".to_string())
            }
        }
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .after_help(AFTER_HELP)
        .infer_long_args(true)
        .arg(Arg::new(options::OPERANDS).num_args(1..))
}

#[cfg(test)]
mod tests {
    use crate::{calc_bsize, Output, Parser};

    use std::path::Path;

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
    fn test_nocreat_causes_failure_when_ofile_doesnt_exist() {
        let args = &["conv=nocreat", "of=not-a-real.file"];
        let settings = Parser::new().parse(args).unwrap();
        assert!(
            Output::new_file(Path::new(settings.outfile.as_ref().unwrap()), &settings).is_err()
        );
    }
}
