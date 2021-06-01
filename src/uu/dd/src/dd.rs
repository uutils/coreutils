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

use std::cmp;
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
    block: Option<usize>,
    unblock: Option<usize>,
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
    InvalidConvBlockUnblockCase,
}

impl std::fmt::Display for InternalError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self
        {
            Self::WrongInputType |
            Self::WrongOutputType =>
                write!(f, "Internal dd error: Wrong Input/Output data type"),
            Self::InvalidConvBlockUnblockCase =>
                write!(f, "Internal dd error: Invalid Conversion, Block, or Unblock data"),
        }
    }
}

impl Error for InternalError {}

struct Input<R: Read>
{
    src: R,
    non_ascii: bool,
    ibs: usize,
    xfer_stats: StatusLevel,
    cflags: IConvFlags,
    iflags: IFlags,
}

impl Input<io::Stdin>
{
    fn new(matches: &getopts::Matches) -> Result<Self, Box<dyn Error>>
    {
        let ibs = parseargs::parse_ibs(matches)?;
        let non_ascii = parseargs::parse_input_non_ascii(matches)?;
        let xfer_stats = parseargs::parse_status_level(matches)?;
        let cflags = parseargs::parse_conv_flag_input(matches)?;
        let iflags = parseargs::parse_iflags(matches)?;
        let skip = parseargs::parse_skip_amt(&ibs, &iflags, matches)?;

        let mut i = Input {
            src: io::stdin(),
            non_ascii,
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
        let non_ascii = parseargs::parse_input_non_ascii(matches)?;
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
                non_ascii,
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

impl<R: Read> Read for Input<R>
{
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize>
    {
        // Read from source, ignore read errors if conv=noerror
        let len = match self.src.read(&mut buf)
        {
            Ok(len) =>
                len,
            Err(e) =>
                if !self.cflags.noerror
                {
                    return Err(e);
                }
                else
                {
                    return Ok(0);
                },
        };

        Ok(len)
    }
}

impl<R: Read> Input<R>
{
    /// Fills to a given size n, which is expected to be 'obs'.
    /// Reads in increments of 'self.ibs'.
    fn fill_n(&mut self, buf: &mut [u8], obs: usize) -> Result<usize, Box<dyn Error>>
    {
        let ibs = self.ibs;
        let mut bytes_read = 0;

        // TODO: Fix this!
        // assert!(obs > ibs);

        for n in 0..(obs/ibs) {
            // fill an ibs-len slice from src
            let this_read = self.read(&mut buf[n*ibs..(n+1)*ibs])?;

            if this_read != 0 {
                bytes_read += this_read;
            } else {
                break;
            }
        }

        Ok(bytes_read)
   }

    /// Force-fills a buffer, ignoring zero-length reads which would otherwise be
    /// interpreted as EOF. Does not continue after errors.
    /// Note: This may never return.
    fn force_fill(&mut self, mut buf: &mut [u8], target_len: usize) -> Result<(), Box<dyn Error>>
    {
        let mut total_len = 0;

        loop
        {
            total_len += self.read(&mut buf)?;

            if total_len == target_len
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

    fn fsync(&mut self) -> io::Result<()>
    {
        self.dst.flush()
    }

    fn fdatasync(&mut self) -> io::Result<()>
    {
        self.dst.flush()
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
            // The following error should only occur if someone
            // mistakenly calls Output::<File>::new() without checking
            // if 'of' has been provided. In this case,
            // Output::<io::stdout>::new() is probably intended.
            Err(Box::new(InternalError::WrongOutputType))
        }
    }

    fn fsync(&mut self) -> io::Result<()>
    {
        self.dst.flush()?;
        self.dst.sync_all()
    }

    fn fdatasync(&mut self) -> io::Result<()>
    {
        self.dst.flush()?;
        self.dst.sync_data()
    }
}

impl Seek for Output<File>
{
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64>
    {
        self.dst.seek(pos)
    }
}

impl Write for Output<File>
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>
    {
        #[inline]
        fn is_sparse(buf: &[u8]) -> bool
        {
            buf.iter()
               .all(|&e| e == 0u8)
        }
        // -----------------------------
        if self.cflags.sparse && is_sparse(buf)
        {
            let seek_amt: i64 = buf.len()
                                   .try_into()
                                   .expect("Internal dd Error: Seek amount greater than signed 64-bit integer");
            self.dst.seek(io::SeekFrom::Current(seek_amt))?;
            Ok(buf.len())
        }
        else
        {
            self.dst.write(buf)
        }
    }

    fn flush(&mut self) -> io::Result<()>
    {
        self.dst.flush()
    }
}

impl Write for Output<io::Stdout>
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
fn pad(buf: &mut Vec<u8>, cbs: usize, pad: u8)
{
    buf.resize(cbs, pad)
}

/// Splits the content of buf into cbs-length blocks
/// Appends padding as specified by conv=block and cbs=N
fn block(mut buf: Vec<u8>, cbs: usize) -> Vec<Vec<u8>>
{
    let mut blocks = buf.split(| &e | e == '\n' as u8)
                    .fold(Vec::new(), | mut blocks, split |
                        {
                            let mut split = split.to_vec();
                            split.resize(cbs, ' ' as u8);
                            blocks.push(split);

                            blocks
                        });

    if let Some(last) = blocks.last()
    {
        if last.iter().all(| &e | e == ' ' as u8)
        {
            blocks.pop();
        }
    }

    blocks
}

// Trims padding from each cbs-length partition of buf
// as specified by conv=unblock and cbs=N
fn unblock(buf: &[u8], cbs: usize)
{
    unimplemented!()
}

#[inline]
fn apply_ct(buf: &mut [u8], ct: &ConversionTable)
{
    for idx in 0..buf.len()
    {
        buf[idx] = ct[buf[idx] as usize];
    }
}

#[inline]
fn perform_swab(buf: &mut [u8])
{
    let mut tmp;

    for base in (1..buf.len()).step_by(2)
    {
        tmp = buf[base];
        buf[base] = buf[base-1];
        buf[base-1] = tmp;
    }
}

fn conv_block_unblock_helper<R: Read, W: Write>(mut buf: Vec<u8>, i: &mut Input<R>, o: &Output<W>) -> Result<Vec<u8>, Box<dyn Error>>
{
    #[inline]
    fn should_block_then_conv<R: Read>(i: &Input<R>) -> bool
    {
        !i.non_ascii
            && i.cflags.block.is_some()
    }
    #[inline]
    fn should_conv_then_block<R: Read>(i: &Input<R>) -> bool
    {
        i.non_ascii
            && i.cflags.block.is_some()
    }
    #[inline]
    fn should_unblock_then_conv<R: Read>(i: &Input<R>) -> bool
    {
        !i.non_ascii
            && i.cflags.unblock.is_some()
    }
    #[inline]
    fn should_conv_then_unblock<R: Read>(i: &Input<R>) -> bool
    {
        i.non_ascii
            && i.cflags.unblock.is_some()
    }
    fn conv_only<R: Read>(i: &Input<R>) -> bool
    {
        i.cflags.ctable.is_some()
            && i.cflags.block.is_none()
            && i.cflags.unblock.is_none()
    }
    // --------------------------------------------------------------------
    if conv_only(&i)
    { // no block/unblock
        let ct = i.cflags.ctable.unwrap();
        apply_ct(&mut buf, &ct);

        Ok(buf)
    }
    else if should_block_then_conv(&i)
    { // ascii input so perform the block first
        let cbs = i.cflags.block.unwrap();

        let mut blocks = block(buf, cbs);

        if let Some(ct) = i.cflags.ctable
        {
            for buf in blocks.iter_mut()
            {
                apply_ct(buf, &ct);
            }
        }

       let blocks = blocks.into_iter()
                           .flatten()
                           .collect();

        Ok(blocks)
    }
    else if should_conv_then_block(&i)
    { // Non-ascii so perform the conversion first
        let cbs = i.cflags.block.unwrap();

        if let Some(ct) = i.cflags.ctable
        {
             apply_ct(&mut buf, &ct);
        }

        let blocks = block(buf, cbs)
            .into_iter()
            .flatten()
            .collect();

        Ok(blocks)
    }
    else if should_unblock_then_conv(&i)
    { // ascii input so perform the unblock first
        let cbs = i.cflags.unblock.unwrap();

        unblock(&mut buf, cbs);

        if let Some(ct) = i.cflags.ctable
        {
             apply_ct(&mut buf, &ct);
        }

        Ok(buf)
    }
    else if should_conv_then_unblock(&i)
    { // Non-ascii input so perform the conversion first
        let cbs = i.cflags.unblock.unwrap();

        if let Some(ct) = i.cflags.ctable
        {
             apply_ct(&mut buf, &ct);
        }

        unblock(&buf, cbs);

        Ok(buf)
    }
    else
    {
        // The following error should not happen, as it results from
        // insufficient command line data. This case should be caught
        // by the parser before making it this far.
        // Producing this error is an alternative to risking an unwrap call
        // on 'cbs' if the required data is not provided.
        Err(Box::new(InternalError::InvalidConvBlockUnblockCase))
    }
}

fn read_write_helper<R: Read, W: Write>(i: &mut Input<R>, o: &mut Output<W>) -> Result<(usize, Vec<u8>), Box<dyn Error>>
{
    #[inline]
    fn is_fast_read<R: Read, W: Write>(i: &Input<R>, o: &Output<W>) -> bool
    {
        // TODO: Enable this once fast_reads are implemented
        false &&
        i.ibs == o.obs
            && !is_conv(i)
            && !is_block(i)
            && !is_unblock(i)
            && !i.cflags.swab
    }
    #[inline]
    fn is_conv<R: Read>(i: &Input<R>) -> bool
    {
        i.cflags.ctable.is_some()
    }
    #[inline]
    fn is_block<R: Read>(i: &Input<R>) -> bool
    {
        i.cflags.block.is_some()
    }
    #[inline]
    fn is_unblock<R: Read>(i: &Input<R>) -> bool
    {
        i.cflags.unblock.is_some()
    }
    // --------------------------------------------------------------------
    if is_fast_read(&i, &o)
    {
        // TODO: fast reads are copies performed
        // directly to output (without creating any buffers)
        // as mentioned in the dd spec.
        unimplemented!()
    }
    else
    {
        // Read
        // let mut buf = Vec::with_capacity(o.obs);
        let mut buf = vec![DEFAULT_FILL_BYTE; o.obs];
        let rlen = i.fill_n(&mut buf, o.obs)?;

        if rlen == 0
        {
            return Ok((0,Vec::new()));
        }


        // Conv etc...
        if i.cflags.swab
        {
            perform_swab(&mut buf[..rlen]);
        }

        if is_conv(&i) || is_block(&i) || is_unblock(&i)
        {
            let buf = conv_block_unblock_helper(buf, i, o)?;
            Ok((rlen, buf))
        }
        else
        {
            Ok((rlen, buf))
        }
    }
}

/// Perform the copy/convert opertaions. Non file backed output version
// Note: Some of dd's functionality depends on whether the output is actually a file. This breaks the Output<Write> abstraction,
// and should be fixed in the future.
fn dd_stdout<R: Read>(mut i: Input<R>, mut o: Output<io::Stdout>) -> Result<(usize, usize), Box<dyn Error>>
{
    let mut bytes_in  = 0;
    let mut bytes_out = 0;

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

    loop
    {
        match read_write_helper(&mut i, &mut o)?
        {
            (0, _) =>
                break,
            (rlen, buf) =>
            {
                let wlen = o.write(&buf)?;

                bytes_in += rlen;
                bytes_out += wlen;
            },
        };

        // Prog
        if let Some(prog_tx) = &prog_tx
        {
            prog_tx.send(bytes_out)?;
        }
    }

    if o.cflags.fsync
    {
        o.fsync()?;
    }
    else if o.cflags.fdatasync
    {
        o.fdatasync()?;
    }

    Ok((bytes_in, bytes_out))
}

/// Perform the copy/convert opertaions. File backed output version
// Note: Some of dd's functionality depends on whether the output is actually a file. This breaks the Output<Write> abstraction,
// and should be fixed in the future.
fn dd_fileout<R: Read>(mut i: Input<R>, mut o: Output<File>) -> Result<(usize, usize), Box<dyn Error>>
{
    let mut bytes_in  = 0;
    let mut bytes_out = 0;

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

    loop
    {
        match read_write_helper(&mut i, &mut o)?
        {
            (0, _) =>
                break,
            (rlen, buf) =>
            {
                let wlen = o.write(&buf)?;

                bytes_in += rlen;
                bytes_out += wlen;
            },
        };

        // Prog
        if let Some(prog_tx) = &prog_tx
        {
            prog_tx.send(bytes_out)?;
        }
    }

    if o.cflags.fsync
    {
        o.fsync()?;
    }
    else if o.cflags.fdatasync
    {
        o.fdatasync()?;
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

