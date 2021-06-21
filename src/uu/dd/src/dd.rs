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
mod dd_unit_tests;

mod parseargs;

mod conversion_tables;
use conversion_tables::*;

use byte_unit::Byte;
// #[macro_use]
use debug_print::debug_println;
use gcd::Gcd;
use getopts;
use signal_hook::consts::signal;
use std::cmp;
use std::convert::TryInto;
use std::error::Error;
use std::env;
use std::fs::{
    File, OpenOptions,
};
use std::io::{
    self, Read, Write,
    Seek,
};
#[cfg(unix)]
use libc;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::sync::{
    Arc, atomic::AtomicUsize, mpsc, atomic::Ordering,
};
use std::thread;
use std::time;

const SYNTAX: &str = "dd [OPERAND]...\ndd OPTION";
const SUMMARY: &str = "convert, and optionally copy, a file";
const LONG_HELP: &str = "";
const BUF_INIT_BYTE: u8 = 0xDD;
const RTN_SUCCESS: i32 = 0;
const RTN_FAILURE: i32 = 1;

// ----- Datatypes -----
struct ProgUpdate
{
    reads_complete: u64,
    reads_partial: u64,
    writes_complete: u64,
    writes_partial: u64,
    bytes_total: u128,
    records_truncated: u32,
    duration: time::Duration,
}

struct ReadStat
{
    reads_complete: u64,
    reads_partial: u64,
    records_truncated: u32,
}
impl std::ops::AddAssign for ReadStat
{
    fn add_assign(&mut self, other: Self)
    {
        *self = Self {
            reads_complete: self.reads_complete + other.reads_complete,
            reads_partial: self.reads_partial + other.reads_partial,
            records_truncated: self.records_truncated + other.records_truncated,
        }
    }
}

struct WriteStat
{
    writes_complete: u64,
    writes_partial: u64,
    bytes_total: u128,
}
impl std::ops::AddAssign for WriteStat
{
    fn add_assign(&mut self, other: Self)
    {
        *self = Self {
            writes_complete: self.writes_complete + other.writes_complete,
            writes_partial: self.writes_partial + other.writes_partial,
            bytes_total: self.bytes_total + other.bytes_total,
        }
    }
}

type Cbs = usize;

/// Stores all Conv Flags that apply to the input
pub struct IConvFlags
{
    ctable: Option<&'static ConversionTable>,
    block: Option<Cbs>,
    unblock: Option<Cbs>,
    swab: bool,
    sync: Option<u8>,
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
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StatusLevel
{
    Progress,
    Noxfer,
    None,
}

/// The value of count=N
/// Defaults to Reads(N)
/// if iflag=count_bytes
/// then becomes Bytes(N)
pub enum CountType
{
    Reads(usize),
    Bytes(usize),
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
    xfer_stats: Option<StatusLevel>,
    count: Option<CountType>,
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

        if let Some(amt) = skip
        {
            let mut buf = vec![BUF_INIT_BYTE; amt];

            i.force_fill(&mut buf, amt)?;
        }

        Ok(i)
    }
}

fn make_unix_iflags(oflags: &IFlags) -> Option<libc::c_int>
{
    let mut flag = 0;

    if oflags.direct
    {
        flag |= libc::O_DIRECT;
    }
    if oflags.directory
    {
        flag |= libc::O_DIRECTORY;
    }
    if oflags.dsync
    {
        flag |= libc::O_DSYNC;
    }
    if oflags.noatime
    {
        flag |= libc::O_NOATIME;
    }
    if oflags.noctty
    {
        flag |= libc::O_NOCTTY;
    }
    if oflags.nofollow
    {
        flag |= libc::O_NOFOLLOW;
    }
    if oflags.nonblock
    {
        flag |= libc::O_NONBLOCK;
    }
    if oflags.sync
    {
        flag |= libc::O_SYNC;
    }

    if flag != 0
    {
        Some(flag)
    }
    else
    {
        None
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
        let count = parseargs::parse_count(&iflags, matches)?;

        if let Some(fname) = matches.opt_str("if")
        {
            let mut src =
            {
                let mut opts = OpenOptions::new();
                opts.read(true);

                if cfg!(unix)
                {
                    if let Some(libc_flags) = make_unix_iflags(&iflags)
                    {
                        opts.custom_flags(libc_flags);
                    }
                }

                opts.open(fname)?
            };

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
                count,
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
        match self.src.read(&mut buf)
        {
            Ok(len) =>
                Ok(len),
            Err(e) =>
                if !self.cflags.noerror
                {
                    Err(e)
                }
                else
                {
                    Ok(0)
                },
        }
    }
}

impl<R: Read> Input<R>
{
    /// Fills a given buffer.
    /// Reads in increments of 'self.ibs'.
    /// The start of each ibs-sized read follows the previous one.
    fn fill_consecutive(&mut self, buf: &mut Vec<u8>) -> Result<ReadStat, Box<dyn Error>>
    {
        let mut reads_complete = 0;
        let mut reads_partial = 0;
        let mut base_idx = 0;

        while base_idx < buf.len()
        {
            let next_blk = cmp::min(base_idx+self.ibs, buf.len());

            match self.read(&mut buf[base_idx..next_blk])?
            {
                rlen if rlen == self.ibs =>
                {
                    base_idx += rlen;
                    reads_complete += 1;
                },
                rlen if rlen > 0 =>
                {
                    base_idx += rlen;
                    reads_partial += 1;
                },
                _ =>
                    break,
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
    /// The start of each ibs-sized read is aligned to multiples of ibs; remaing space is filled with the 'pad' byte.
    fn fill_blocks(&mut self, buf: &mut Vec<u8>, pad: u8) -> Result<ReadStat, Box<dyn Error>>
    {
        let mut reads_complete = 0;
        let mut reads_partial = 0;
        let mut base_idx = 0;

        while base_idx < buf.len()
        {
            let next_blk = cmp::min(base_idx+self.ibs, buf.len());
            let plen = next_blk - base_idx;

            match self.read(&mut buf[base_idx..next_blk])?
            {
                0 =>
                    break,
                rlen if rlen < plen =>
                {
                    reads_partial += 1;
                    let padding = vec![pad; plen-rlen];
                    buf.splice(base_idx+rlen..next_blk, padding.into_iter());
                },
                _ =>
                {
                    reads_complete += 1;
                },
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
    fn force_fill(&mut self, buf: &mut [u8], target_len: usize) -> Result<usize, Box<dyn Error>>
    {
        let mut base_idx = 0;
        while base_idx < target_len
        {
            base_idx += self.read(&mut buf[base_idx..target_len])?;
        }

    Ok(base_idx)
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

fn make_unix_oflags(oflags: &OFlags) -> Option<libc::c_int>
{
    let mut flag = 0;

    if oflags.direct
    {
        flag |= libc::O_DIRECT;
    }
    if oflags.directory
    {
        flag |= libc::O_DIRECTORY;
    }
    if oflags.dsync
    {
        flag |= libc::O_DSYNC;
    }
    if oflags.noatime
    {
        flag |= libc::O_NOATIME;
    }
    if oflags.noctty
    {
        flag |= libc::O_NOCTTY;
    }
    if oflags.nofollow
    {
        flag |= libc::O_NOFOLLOW;
    }
    if oflags.nonblock
    {
        flag |= libc::O_NONBLOCK;
    }
    if oflags.sync
    {
        flag |= libc::O_SYNC;
    }

    if flag != 0
    {
        Some(flag)
    }
    else
    {
        None
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
            let mut dst = {
                let mut opts = OpenOptions::new();
                opts.write(true)
                    .append(oflags.append)
                    .create_new(cflags.excl || !cflags.nocreat)
                    .truncate(!cflags.notrunc);

                if cfg!(unix)
                {
                    if let Some(libc_flags) = make_unix_oflags(&oflags)
                    {
                        opts.custom_flags(libc_flags);
                    }
                }

                opts.open(fname)?
            };

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

impl Output<io::Stdout>
{
    fn write_blocks(&mut self, buf: Vec<u8>) -> io::Result<WriteStat>
    {
        let mut writes_complete = 0;
        let mut writes_partial = 0;
        let mut base_idx = 0;

        while base_idx < buf.len()
        {
            let next_blk = cmp::min(base_idx+self.obs, buf.len());
            let plen = next_blk - base_idx;

            match self.write(&buf[base_idx..next_blk])?
            {
                wlen if wlen < plen =>
                {
                    writes_partial += 1;
                    base_idx += wlen;
                },
                wlen =>
                {
                    writes_complete += 1;
                    base_idx += wlen;
                },
            }
        }

        Ok(WriteStat {
            writes_complete,
            writes_partial,
            bytes_total: base_idx.try_into().unwrap_or(0u128),
        })
   }
}

impl Output<File>
{
    fn write_blocks(&mut self, buf: Vec<u8>) -> io::Result<WriteStat>
    {
        let mut writes_complete = 0;
        let mut writes_partial = 0;
        let mut base_idx = 0;

        while base_idx < buf.len()
        {
            let next_blk = cmp::min(base_idx+self.obs, buf.len());
            let plen = next_blk - base_idx;

            match self.write(&buf[base_idx..next_blk])?
            {
                wlen if wlen < plen =>
                {
                    writes_partial += 1;
                    base_idx += wlen;
                },
                wlen =>
                {
                    writes_complete += 1;
                    base_idx += wlen;
                },
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
fn block(buf: Vec<u8>, cbs: usize, rstat: &mut ReadStat) -> Vec<Vec<u8>>
{
    let mut blocks = buf.split(| &e | e == '\n' as u8)
                    .fold(Vec::new(), | mut blocks, split |
                        {
                            let mut split = split.to_vec();
                            if split.len() > cbs
                            {
                                rstat.records_truncated += 1;
                            }
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

/// Trims padding from each cbs-length partition of buf
/// as specified by conv=unblock and cbs=N
fn unblock(buf: Vec<u8>, cbs: usize) -> Vec<u8>
{
    // Local Helper Fns ----------------------------------------------------
    #[inline]
    fn build_blocks(buf: Vec<u8>, cbs: usize) -> Vec<Vec<u8>>
    {
        let mut blocks = Vec::new();
        let mut curr = buf;
        let mut next;
        let mut width;

        while !curr.is_empty()
        {
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
        .fold(Vec::new(), | mut unblocks, mut block | {
            let block = if let Some(last_char_idx) = block.iter().rposition(| &e | e != ' ' as u8)
            {
                block.truncate(last_char_idx+1);
                block.push('\n' as u8);

                block
            }
            else if let Some(32u8/* ' ' as u8 */) = block.get(0)
            {
                vec!['\n' as u8]
            }
            else
            {
                block
            };

            unblocks.push(block);

            unblocks
        })
        .into_iter()
        .flatten()
        .collect()
}

fn conv_block_unblock_helper<R: Read>(mut buf: Vec<u8>, i: &mut Input<R>, rstat: &mut ReadStat) -> Result<Vec<u8>, Box<dyn Error>>
{
    // Local Predicate Fns -------------------------------------------------
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
    // Local Helper Fns ----------------------------------------------------
    #[inline]
    fn apply_ct(buf: &mut [u8], ct: &ConversionTable)
    {
        for idx in 0..buf.len()
        {
            buf[idx] = ct[buf[idx] as usize];
        }
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

        let mut blocks = block(buf, cbs, rstat);

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

        let blocks = block(buf, cbs, rstat)
            .into_iter()
            .flatten()
            .collect();

        Ok(blocks)
    }
    else if should_unblock_then_conv(&i)
    { // ascii input so perform the unblock first
        let cbs = i.cflags.unblock.unwrap();

        let mut buf = unblock(buf, cbs);

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

        let buf = unblock(buf, cbs);

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

fn read_helper<R: Read>(i: &mut Input<R>, bsize: usize) -> Result<(ReadStat, Vec<u8>), Box<dyn Error>>
{
    // Local Predicate Fns -----------------------------------------------
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
    // Local Helper Fns -------------------------------------------------
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
   // ------------------------------------------------------------------
   // Read
   let mut buf = vec![BUF_INIT_BYTE; bsize];
   let mut rstat = match i.cflags.sync
   {
       Some(ch) =>
           i.fill_blocks(&mut buf, ch)?,
       _ =>
           i.fill_consecutive(&mut buf)?,
   };
   // Return early if no data
   if rstat.reads_complete == 0 && rstat.reads_partial == 0
   {
       return Ok((rstat,buf));
   }

   // Perform any conv=x[,x...] options
   if i.cflags.swab
   {
       perform_swab(&mut buf);
   }
   if is_conv(&i) || is_block(&i) || is_unblock(&i)
   {
       let buf = conv_block_unblock_helper(buf, i, &mut rstat)?;
       Ok((rstat, buf))
   }
   else
   {
       Ok((rstat, buf))
   }
}

fn print_io_lines(update: &ProgUpdate)
{
    eprintln!("{}+{} records in", update.reads_complete, update.reads_partial);
    if update.records_truncated > 0
    {
        eprintln!("{} truncated records", update.records_truncated);
    }
    eprintln!("{}+{} records out", update.writes_complete, update.writes_partial);
}
fn make_prog_line(update: &ProgUpdate) -> String
{
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

    format!("{} bytes ({}, {}) copied, {:.1} s, {}/s",
            update.bytes_total,
            btotal_metric,
            btotal_bin,
            update.duration.as_secs_f64(),
            xfer_rate
    ).to_string()
}
fn reprint_prog_line(update: &ProgUpdate)
{
    eprint!("\r{}", make_prog_line(update));
}
fn print_prog_line(update: &ProgUpdate)
{
    eprintln!("{}", make_prog_line(update));
}
fn print_xfer_stats(update: &ProgUpdate)
{
    print_io_lines(update);
    print_prog_line(update);

}

/// Generate a progress updater that tracks progress, receives updates, and responds to signals.
fn gen_prog_updater(rx: mpsc::Receiver<ProgUpdate>, xfer_stats: Option<StatusLevel>) -> impl Fn() -> ()
{
    // --------------------------------------------------------------
    fn posixly_correct() -> bool
    {
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
        if !posixly_correct()
        {
            if let Err(e) = signal_hook::flag::register_usize(signal::SIGUSR1, sigval.clone(), SIGUSR1_USIZE)
            {
                debug_println!("Internal dd Warning: Unable to register SIGUSR1 handler \n\t{}", e);
            }
        }

        loop
        {
            // Wait for update
            let update = match (rx.recv(), xfer_stats)
            {
                (Ok(update), Some(StatusLevel::Progress)) =>
                {
                    reprint_prog_line(&update);

                    update
                },
                (Ok(update), _) =>
                {
                    update
                },
                (Err(_), _) =>
                    // recv only fails permenantly
                    break,
            };
            // Handle signals
            match sigval.load(Ordering::Relaxed)
            {
                SIGUSR1_USIZE =>
                {
                    print_xfer_stats(&update);
                },
                _ => {/* no signals recv'd */},
            };
        }
    }
}

/// Calculate a 'good' internal buffer size.
/// For performance of the read/write functions, the buffer should hold
/// both an itegral number of reads and an itegral number of writes. For
/// sane real-world memory use, it should not be too large. I believe
/// the least common multiple is a good representation of these interests.
#[inline]
fn calc_bsize(ibs: usize, obs: usize) -> usize
{
    let gcd = Gcd::gcd(ibs, obs);
    let lcm = (ibs/gcd)*obs;

    lcm
}

/// Calculate the buffer size appropriate for this loop iteration, respecting
/// a count=N if present.
fn calc_loop_bsize(count: &Option<CountType>, rstat: &ReadStat, wstat: &WriteStat, ibs: usize, ideal_bsize: usize) -> usize
{
    match count
    {
        Some(CountType::Reads(rmax)) =>
        {
            let rmax: u64 = (*rmax).try_into().unwrap();
            let rsofar = rstat.reads_complete + rstat.reads_partial;
            let rremain: usize = (rmax - rsofar).try_into().unwrap();
            cmp::min(ideal_bsize, rremain*ibs)
        },
        Some(CountType::Bytes(bmax)) =>
        {
            let bmax: u128 = (*bmax).try_into().unwrap();
            let bremain: usize = (bmax - wstat.bytes_total).try_into().unwrap();
            cmp::min(ideal_bsize, bremain)
        },
        None =>
            ideal_bsize,
    }
}

/// Decide if the current progress is below a count=N limit or return
/// true if no such limit is set.
fn below_count_limit(count: &Option<CountType>, rstat: &ReadStat, wstat: &WriteStat) -> bool
{
    match count
    {
        Some(CountType::Reads(n)) =>
        {
            let n = (*n).try_into().unwrap();
            // debug_assert!(rstat.reads_complete + rstat.reads_partial >= n);
            rstat.reads_complete + rstat.reads_partial <= n
        },
        Some(CountType::Bytes(n)) =>
        {
            let n = (*n).try_into().unwrap();
            // debug_assert!(wstat.bytes_total >= n);
            wstat.bytes_total <= n
        },
        None =>
            true,
    }
}

/// Perform the copy/convert opertaions. Stdout version
// Note: Some of dd's functionality depends on whether the output is actually a file. This breaks the Output<Write> abstraction,
// and should be fixed in the future.
fn dd_stdout<R: Read>(mut i: Input<R>, mut o: Output<io::Stdout>) -> Result<(), Box<dyn Error>>
{
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

    while below_count_limit(&i.count, &rstat, &wstat)
    {
        // Read/Write
        let loop_bsize = calc_loop_bsize(&i.count, &rstat, &wstat, i.ibs, bsize);
        match read_helper(&mut i, loop_bsize)?
        {
            (ReadStat { reads_complete: 0, reads_partial: 0, .. }, _) =>
                break,
            (rstat_update, buf) =>
            {
                let wstat_update = o.write_blocks(buf)?;

                rstat += rstat_update;
                wstat += wstat_update;
           },
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

    if o.cflags.fsync
    {
        o.fsync()?;
    }
    else if o.cflags.fdatasync
    {
        o.fdatasync()?;
    }

    match i.xfer_stats
    {
        Some(StatusLevel::Noxfer) |
        Some(StatusLevel::None) => {},
        _ =>
            print_xfer_stats(&ProgUpdate {
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

/// Perform the copy/convert opertaions. File backed output version
// Note: Some of dd's functionality depends on whether the output is actually a file. This breaks the Output<Write> abstraction,
// and should be fixed in the future.
fn dd_fileout<R: Read>(mut i: Input<R>, mut o: Output<File>) -> Result<(), Box<dyn Error>>
{
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

    while below_count_limit(&i.count, &rstat, &wstat)
    {
        // Read/Write
        let loop_bsize = calc_loop_bsize(&i.count, &rstat, &wstat, i.ibs, bsize);
        match read_helper(&mut i, loop_bsize)?
        {
            (ReadStat { reads_complete: 0, reads_partial: 0, .. }, _) =>
                break,
            (rstat_update, buf) =>
            {
                let wstat_update = o.write_blocks(buf)?;

                rstat += rstat_update;
                wstat += wstat_update;
            },
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

    if o.cflags.fsync
    {
        o.fsync()?;
    }
    else if o.cflags.fdatasync
    {
        o.fdatasync()?;
    }

    match i.xfer_stats
    {
        Some(StatusLevel::Noxfer) |
        Some(StatusLevel::None) => {},
        _ =>
            print_xfer_stats(&ProgUpdate {
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
                "iflag",
                "read as per the comma separated symbol list of flags",
                "FLAG"
            )
            .optopt(
                "",
                "oflag",
                "write as per the comma separated symbol list of flags",
                "FLAG"
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
            .optopt(
                "",
                "status",
                "Specify the amount of information printed.  If this operand is
     given multiple times, the last one takes precedence.  The LEVEL
     value can be one of the following:

     ‘none’
          Do not print any informational or warning messages to stderr.
          Error messages are output as normal.

     ‘noxfer’
          Do not print the final transfer rate and volume statistics
          that normally make up the last status line.

     ‘progress’
          Print the transfer rate and volume statistics on stderr, when
          processing each input block.  Statistics are output on a
          single line at most once every second, but updates can be
          delayed when waiting on I/O.

     Transfer information is normally output to stderr upon receipt of
     the ‘INFO’ signal or when ‘dd’ exits, and defaults to the following
     form in the C locale:

          7287+1 records in
          116608+0 records out
          59703296 bytes (60 MB, 57 MiB) copied, 0.0427974 s, 1.4 GB/s

     The notation ‘W+P’ stands for W whole blocks and P partial blocks.
     A partial block occurs when a read or write operation succeeds but
     transfers less data than the block size.  An additional line like
     ‘1 truncated record’ or ‘10 truncated records’ is output after the
     ‘records out’ line if ‘conv=block’ processing truncated one or more
     input records.",
                "LEVEL"
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
        Ok(_) =>
        {
           RTN_SUCCESS
        },
        Err(e) =>
        {
            debug_println!("dd exiting with error:\n\t{}", e);
            RTN_FAILURE
        },
    }
}

