// This file is part of the uutils coreutils package.
//
// (c) Tyler Steele <tyler.steele@protonmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (T0DO)

#[macro_use]
extern crate uucore;

#[cfg(test)]
mod dd_test;

mod parseargs;

mod conversion_tables;
use conversion_tables::*;

use std::convert::TryInto;
use std::error::Error;
use std::fs::{
    File, OpenOptions,
};
use getopts;
use std::io::{
    self, Read, Write,
    Seek,
};
use std::sync::mpsc;
use std::thread;

const SYNTAX: &str = "dd [OPERAND]...\ndd OPTION";
const SUMMARY: &str = "convert, and optionally copy, a file";
const LONG_HELP: &str = "";

const DEFAULT_FILL_BYTE: u8 = 0xDD;
const DEFAULT_SKIP_TRIES: u8 = 3;

const RTN_SUCCESS: i32 = 0;
const RTN_FAILURE: i32 = 1;

// ----- Datatypes -----
enum SrcStat
{
    Read(usize),
    EOF,
}

/// Stores all Conv Flags that apply to the input
pub struct IConvFlags
{
    ctable: Option<&'static ConversionTable>,
    cbs: Option<usize>,
    block: bool,
    unblock: bool,
    swab: bool,
    sync: bool,
    noerror: bool,
}

/// Stores all Conv Flags that apply to the output
#[derive(Debug, PartialEq)]
pub struct OConvFlags
{
    sparse: bool,
    excl: bool,
    nocreat: bool,
    notrunc: bool,
    fdatasync: bool,
    fsync: bool,
}

/// Stores all Flags that apply to the input
pub struct IFlags
{
    cio: bool,
    direct: bool,
    directory: bool,
    dsync: bool,
    sync: bool,
    nocache: bool,
    nonblock: bool,
    noatime: bool,
    noctty: bool,
    nofollow: bool,
    nolinks: bool,
    binary: bool,
    text: bool,
    fullblock: bool,
    count_bytes: bool,
    skip_bytes: bool,
}

/// Stores all Flags that apply to the output
pub struct OFlags
{
    append: bool,
    cio: bool,
    direct: bool,
    directory: bool,
    dsync: bool,
    sync: bool,
    nocache: bool,
    nonblock: bool,
    noatime: bool,
    noctty: bool,
    nofollow: bool,
    nolinks: bool,
    binary: bool,
    text: bool,
    seek_bytes: bool,
}

/// The value of the status cl-option.
/// Controls printing of transfer stats
#[derive(PartialEq)]
pub enum StatusLevel
{
    Progress,
    Noxfer,
    None,
}

#[derive(Debug)]
enum InternalError
{
    WrongInputType,
    WrongOutputType,
}

impl std::fmt::Display for InternalError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self
        {
            Self::WrongInputType |
            Self::WrongOutputType =>
                write!(f, "Internal dd error"),
        }
    }
}

impl Error for InternalError {}

struct Input<R: Read>
{
    src: R,
    ibs: usize,
    xfer_stats: StatusLevel,
    cflags: IConvFlags,
    iflags: IFlags,
}

impl<R: Read> Read for Input<R>
{
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize>
    {
        let len = self.src.read(&mut buf)?;

        if let Some(ct) = self.cflags.ctable
        {
            for idx in 0..len
            {
                buf[idx] = ct[buf[idx] as usize];
            }
        }

        if self.cflags.swab
        {
            let mut tmp = DEFAULT_FILL_BYTE;

            for base in (1..len).step_by(2)
            {
                tmp = buf[base];
                buf[base] = buf[base-1];
                buf[base-1] = tmp;
            }
        }

        Ok(len)
    }
}

impl Input<io::Stdin>
{
    fn new(matches: &getopts::Matches) -> Result<Self, Box<dyn Error>>
    {
        let ibs = parseargs::parse_ibs(matches)?;
        let xfer_stats = parseargs::parse_status_level(matches)?;
        let cflags = parseargs::parse_conv_flag_input(matches)?;
        let iflags = parseargs::parse_iflags(matches)?;
        let skip = parseargs::parse_skip_amt(&ibs, &iflags, matches)?;

        let mut i = Input {
            src: io::stdin(),
            ibs,
            xfer_stats,
            cflags,
            iflags,
        };

        if let Some(amt) = skip
        {
            let mut buf = vec![DEFAULT_FILL_BYTE; amt];

            i.force_fill(&mut buf, amt)?;
        }

        Ok(i)
    }
}

impl Input<File>
{
    fn new(matches: &getopts::Matches) -> Result<Self, Box<dyn Error>>
    {
        let ibs = parseargs::parse_ibs(matches)?;
        let xfer_stats = parseargs::parse_status_level(matches)?;
        let cflags = parseargs::parse_conv_flag_input(matches)?;
        let iflags = parseargs::parse_iflags(matches)?;
        let skip = parseargs::parse_skip_amt(&ibs, &iflags, matches)?;

        if let Some(fname) = matches.opt_str("if")
        {
            let mut src = File::open(fname)?;

            if let Some(amt) = skip
            {
                let amt: u64 = amt.try_into()?;
                src.seek(io::SeekFrom::Start(amt))?;
            }

            let i = Input {
                src,
                ibs,
                xfer_stats,
                cflags,
                iflags,
            };

            Ok(i)
        }
        else
        {
            Err(Box::new(InternalError::WrongInputType))
        }

    }
}

impl<R: Read> Input<R>
{
    fn fill_n(&mut self, buf: &mut [u8], obs: usize) -> Result<SrcStat, Box<dyn Error>>
    {
        let ibs = self.ibs;
        let mut bytes_read = 0;

        for n in 0..(obs/ibs) {
            // fill an ibs-len slice from src
            let this_read = self.read(&mut buf[n*ibs..(n+1)*ibs])?;

            if this_read != 0 {
                bytes_read += this_read;
            } else {
                break;
            }
        }

        if bytes_read != 0 {
            Ok(SrcStat::Read(bytes_read))
        } else {
            Ok(SrcStat::EOF)
        }
    }

    fn force_fill(&mut self, mut buf: &mut [u8], len: usize) -> Result<(), Box<dyn Error>>
    {
        let mut total_len = 0;

        loop
        {
            total_len += self.read(&mut buf)?;

            if total_len == len
            {
                return Ok(());
            }
        }
    }
}

struct Output<W: Write>
{
    dst: W,
    obs: usize,
    cflags: OConvFlags,
    oflags: OFlags,
}

impl Output<io::Stdout> {
    fn new(matches: &getopts::Matches) -> Result<Self, Box<dyn Error>>
    {
        let obs = parseargs::parse_obs(matches)?;
        let cflags = parseargs::parse_conv_flag_output(matches)?;
        let oflags = parseargs::parse_oflags(matches)?;

        Ok(Output {
                dst: io::stdout(),
                obs,
                cflags,
                oflags,
        })
    }
}

impl Output<File> {
    fn new(matches: &getopts::Matches) -> Result<Self, Box<dyn Error>>
    {
        let obs = parseargs::parse_obs(matches)?;
        let cflags = parseargs::parse_conv_flag_output(matches)?;
        let oflags = parseargs::parse_oflags(matches)?;
        let seek = parseargs::parse_seek_amt(&obs, &oflags, matches)?;

        if let Some(fname) = matches.opt_str("of")
        {
            let mut dst = OpenOptions::new()
                .write(true)
                .create(!cflags.nocreat)
                .truncate(!cflags.notrunc)
                .open(fname)?;

            if let Some(amt) = seek
            {
                let amt: u64 = amt.try_into()?;
                dst.seek(io::SeekFrom::Start(amt))?;
            }

            Ok(Output {
                dst,
                obs,
                cflags,
                oflags,
            })
        }
        else
        {
            Err(Box::new(InternalError::WrongOutputType))
        }
    }
}

impl Seek for Output<File>
{
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64>
    {
        self.dst.seek(pos)
    }
}

impl<W: Write> Write for Output<W>
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>
    {
            self.dst.write(buf)
    }

    fn flush(&mut self) -> io::Result<()>
    {
        self.dst.flush()
    }
}

fn is_sparse(buf: &[u8]) -> bool
{
    buf.iter().all(| &e | e == 0)
}

fn gen_prog_updater(rx: mpsc::Receiver<usize>) -> impl Fn() -> ()
{
    move || {

        // TODO: Replace ?? with accurate info
        print!("\rProgress ({}/??)", 0);

        loop
        {
            match rx.recv()
            {
                Ok(wr_total) => {
                    print!("\rProgress ({}/??)", wr_total);
                },
                Err(_) => {
                    println!("");
                    break
                },
            }
        }
    }
}

#[inline]
fn dd_read_helper<R: Read, W: Write>(mut buf: &mut [u8], i: &mut Input<R>, o: &Output<W>) -> Result<SrcStat, Box<dyn Error>>
{
    match i.fill_n(&mut buf, o.obs)
    {
        Ok(ss) =>
            Ok(ss),
        Err(e) =>
            if !i.cflags.noerror
            {
                return Err(e);
            }
            else
            {
                Ok(SrcStat::Read(0))
            },
    }
}

/// Perform the copy/convert opertaions. Non file backed output version
// Note: Some of dd's functionality depends on whether the output is actually a file. This breaks the Output<Write> abstraction,
// and should be fixed in the future.
fn dd_stdout<R: Read>(mut i: Input<R>, mut o: Output<io::Stdout>) -> Result<(usize, usize), Box<dyn Error>>
{
    let prog_tx = if i.xfer_stats == StatusLevel::Progress
    {
        let (prog_tx, prog_rx) = mpsc::channel();

        thread::spawn(gen_prog_updater(prog_rx));

        Some(prog_tx)
    }
    else
    {
        None
    };

    let mut bytes_in  = 0;
    let mut bytes_out = 0;

    loop
    {
        let mut buf = vec![DEFAULT_FILL_BYTE; o.obs];

        // Read
        let r_len = match dd_read_helper(&mut buf, &mut i, &o)?
        {
            SrcStat::Read(0) =>
                continue,
            SrcStat::Read(len) =>
            {
                bytes_in += len;
                len
            },
            SrcStat::EOF =>
                break,
        };

        // Write
        let w_len = o.write(&buf[..r_len])?;

        // Prog
        bytes_out += w_len;

        if let Some(prog_tx) = &prog_tx
        {
            prog_tx.send(bytes_out)?;
        }
    }

    if o.cflags.fsync || o.cflags.fdatasync
    {
        o.flush()?;
    }

    Ok((bytes_in, bytes_out))
}

/// Perform the copy/convert opertaions. File backed output version
// Note: Some of dd's functionality depends on whether the output is actually a file. This breaks the Output<Write> abstraction,
// and should be fixed in the future.
fn dd_fileout<R: Read>(mut i: Input<R>, mut o: Output<File>) -> Result<(usize, usize), Box<dyn Error>>
{
    let prog_tx = if i.xfer_stats == StatusLevel::Progress
    {
        let (prog_tx, prog_rx) = mpsc::channel();

        thread::spawn(gen_prog_updater(prog_rx));

        Some(prog_tx)
    }
    else
    {
        None
    };

    let mut bytes_in  = 0;
    let mut bytes_out = 0;

    loop
    {
        let mut buf = vec![DEFAULT_FILL_BYTE; o.obs];

        // Read
        let r_len = match dd_read_helper(&mut buf, &mut i, &o)?
        {
            SrcStat::Read(0) =>
                continue,
            SrcStat::Read(len) =>
            {
                bytes_in += len;
                len
            },
            SrcStat::EOF =>
                break,
        };


        // Write
        let w_len = if o.cflags.sparse && is_sparse(&buf)
        {
            let seek_amt: i64 = r_len.try_into()?;
            o.seek(io::SeekFrom::Current(seek_amt))?;
            r_len
        }
        else
        {
            o.write(&buf[..r_len])?
        };

        // Prog
        bytes_out += w_len;

        if let Some(prog_tx) = &prog_tx
        {
            prog_tx.send(bytes_out)?;
        }
    }

    if o.cflags.fsync
    {
        o.flush()?;
        o.dst.sync_all()?;
    }
    else if o.cflags.fdatasync
    {
        o.flush()?;
        o.dst.sync_data()?;
    }

    Ok((bytes_in, bytes_out))
}

#[macro_export]
macro_rules! build_app (
    () =>
    {
        app!(SYNTAX, SUMMARY, LONG_HELP)
            .optopt(
                "",
                "skip",
                "Skip N ‘ibs’-byte blocks in the input file before copying. If ‘iflag=skip_bytes’ is specified, N is interpreted as a byte count rather than a block count.",
                "N"
            )
            .optopt(
                "",
                "seek",
                "Skip N ‘obs’-byte blocks in the input file before copying. If ‘oflag=skip_bytes’ is specified, N is interpreted as a byte count rather than a block count.",
                "N"
            )
            .optopt(
                "",
                "count",
                "Copy N ‘ibs’-byte blocks from the input file, instead of everything until the end of the file. if ‘iflag=count_bytes’ is specified, N is interpreted as a byte count rather than a block count. Note if the input may return short reads as could be the case when reading
     from a pipe for example, ‘iflag=fullblock’ will ensure that ‘count=’ corresponds to complete input blocks rather than the traditional POSIX specified behavior of counting input read operations.",
                "BYTES"
            )
            .optopt(
                "",
                "bs",
                "Set both input and output block sizes to BYTES. This makes ‘dd’ read and write BYTES per block, overriding any ‘ibs’ and ‘obs’ settings. In addition, if no data-transforming ‘conv’ option is specified, input is copied to the output as soon as it’s read, even
     if it is smaller than the block size.",
                "BYTES"
            )
            .optopt(
                "",
                "if",
                "Read from FILE instead of standard input.",
                "FILE"
            )
            .optopt(
                "",
                "ibs",
                "Set the input block size to BYTES. This makes ‘dd’ read BYTES per block. The default is 512 bytes.",
                "BYTES"
            )
            .optopt(
                "",
                "of",
                "Write to FILE instead of standard output. Unless ‘conv=notrunc’ is given, ‘dd’ truncates FILE to zero bytes (or the size specified with ‘seek=’).",
                "FILE"
            )
            .optopt(
                "",
                "obs",
                "Set the output block size to BYTES. This makes ‘dd’ write BYTES per block. The default is 512 bytes.",
                "BYTES"
            )
            .optopt(
                "",
                "conv",
                "Convert the file as specified by the CONVERSION argument(s). (No spaces around any comma(s).)",
                "OPT[,OPT]..."
            )
            .optopt(
                "",
                "cbs",
                "Set the conversion block size to BYTES. When converting variable-length records to fixed-length ones (‘conv=block’) or the reverse (‘conv=unblock’), use BYTES as the fixed record length.",
                "BYTES"
            )
    }
);

fn append_dashes_if_not_present(mut acc: Vec<String>, s: &String) -> Vec<String>
{
    if Some("--") == s.get(0..=1) {
        acc
    } else {
        acc.push(format!("--{}", s));
        acc
    }
}

pub fn uumain(args: impl uucore::Args) -> i32
{
    let dashed_args = args.collect_str()
                          .iter()
                          .fold(Vec::new(), append_dashes_if_not_present);

    let matches = build_app!().parse(dashed_args);

    let result = match (matches.opt_present("if"), matches.opt_present("of"))
    {
        (true, true) =>
        {
            let i = Input::<File>::new(&matches)
                .expect("TODO: Return correct error code");
            let o = Output::<File>::new(&matches)
                .expect("TODO: Return correct error code");

            dd_fileout(i,o)
        },
        (true, false) =>
        {
            let i = Input::<File>::new(&matches)
                .expect("TODO: Return correct error code");
            let o = Output::<io::Stdout>::new(&matches)
                .expect("TODO: Return correct error code");

            dd_stdout(i,o)
        },
        (false, true) =>
        {
            let i = Input::<io::Stdin>::new(&matches)
                .expect("TODO: Return correct error code");
            let o = Output::<File>::new(&matches)
                .expect("TODO: Return correct error code");

            dd_fileout(i,o)
        },
        (false, false) =>
        {
            let i = Input::<io::Stdin>::new(&matches)
                .expect("TODO: Return correct error code");
            let o = Output::<io::Stdout>::new(&matches)
                .expect("TODO: Return correct error code");

            dd_stdout(i,o)
        },
    };

    match result
    {
        Ok((b_in, b_out)) =>
        {
            // TODO: Print final xfer stats
            // print_stats(b_in, b_out);

            RTN_SUCCESS
        },
        Err(_) =>
            RTN_FAILURE,
    }
}

