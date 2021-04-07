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
mod test_dd_internal;

mod parseargs;
mod conversion_tables;

use conversion_tables::*;
use parseargs::*;

use std::error::Error;
use std::fs::File;
use std::io::{
    self, Read, Write,
};
use std::sync::mpsc;
use std::thread;
use getopts;

const SYNTAX: &str = "dd [OPERAND]...\ndd OPTION";
const SUMMARY: &str = "convert, and optionally copy, a file";
const LONG_HELP: &str = "";

const RTN_SUCCESS: i32 = 0;
const RTN_FAILURE: i32 = 1;

// ----- Datatypes -----
enum SrcStat
{
    Read(usize),
    EOF,
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
        write!(f, "Internal dd error")
    }
}

impl Error for InternalError {}

struct Input<R: Read>
{
    src: R,
    ibs: usize,
}

impl<R: Read> Read for Input<R>
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>
    {
        self.src.read(buf)
    }
}

impl Input<io::Stdin>
{
    fn new(matches: &getopts::Matches) -> Result<Self, Box<dyn Error>>
    {
        let ibs: usize = parseargs::parse_ibs(matches)?;

        Ok(
            Input {
                src: io::stdin(),
                ibs,
            }
        )

    }
}

impl Input<File>
{
    fn new(matches: &getopts::Matches) -> Result<Self, Box<dyn Error>>
    {
        let ibs: usize = parseargs::parse_ibs(matches)?;

        if let Some(fname) = matches.opt_str("if")
        {
            Ok(Input {
                src: File::open(fname)?,
                ibs,
            })
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
}

struct Output<W: Write>
{
    dst: W,
    obs: usize,
}

impl Output<io::Stdout> {
    fn new(matches: &getopts::Matches) -> Result<Self, Box<dyn Error>>
    {
        let obs: usize = parseargs::parse_obs(matches)?;

        Ok(
            Output {
                dst: io::stdout(),
                obs,
            }
        )
    }
}

impl Output<File> {
    fn new(matches: &getopts::Matches) -> Result<Self, Box<dyn Error>>
    {
        let obs: usize = parseargs::parse_obs(matches)?;

        if let Some(fname) = matches.opt_str("if")
        {
            Ok(Output {
                dst: File::open(fname)?,
                obs,
            })
        }
        else
        {
            Err(Box::new(InternalError::WrongOutputType))
        }
    }
}

impl<W: Write> Write for Output<W>
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>
    {
        if let Some(ct) = self.conv_table
        {
            let mut cbuf = vec![0; buf.len()];
           
            for (idx, byte) in buf.iter().enumerate()
            {
                cbuf[idx] = ct[*byte as usize]
            }

            self.dst.write(&cbuf)
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

struct Options
{
    conv: Option<ConversionOptions>,
    status_level: StatusLevel,
    // ...
}

struct ConversionOptions
{
    table: Option<ConversionTable>,
    block: bool,
    unblock: bool,
    lcase: bool,
    ucase: bool,
    sparse: bool,
    swab: bool,
    sync: bool,
}

#[derive(PartialEq)]
enum StatusLevel
{
    Progress,
    Noxfer,
    None,
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

fn dd<R: Read, W: Write>(mut i: Input<R>, mut o: Output<W>, opts: Options) -> Result<(usize, usize), Box<dyn Error>>
{
    let prog_tx = if opts.status_level == StatusLevel::Progress
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
        let mut buf = vec![0xDD; o.obs];
        let r_len =
            match i.fill_n(&mut buf, o.obs)? {
                SrcStat::Read(len) =>
                {
                    bytes_in += len;
                    len
                },
                SrcStat::EOF =>
                    break,
        };

        let w_len = o.write(&buf[..r_len])?;

        // TODO: Some flag (sync?) controls this behaviour
        // o.flush()?;

        bytes_out += w_len;

        if let Some(prog_tx) = &prog_tx
        {
            prog_tx.send(bytes_out)?;
        }
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
                "if",
                "The input file",
                "FILE"
            )
            .optopt(
                "",
                "ibs",
                "read up to BYTES bytes at a time (default: 512)",
                "BYTES"
            )
            .optopt(
                "",
                "of",
                "The output file",
                "FILE"
            )
            .optopt(
                "",
                "obs",
                "write BYTES bytes at a time (default: 512)",
                "BYTES"
            )
            .optopt(
                "",
                "conv",
                "One or more conversion options as a comma-serparated list",
                "OPT[,OPT]..."
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

    let opts = parse_options(&matches)
        .expect("TODO: Return correct error code");

    let result = match (matches.opt_present("if"), matches.opt_present("of"))
    {
        (true, true) =>
        {
            let i = Input::<File>::new(&matches)
                .expect("TODO: Return correct error code");
            let o = Output::<File>::new(&matches)
                .expect("TODO: Return correct error code");

            dd(i, o, opts)
        },
        (true, false) =>
        {
            let i = Input::<File>::new(&matches)
                .expect("TODO: Return correct error code");
            let o = Output::<io::Stdout>::new(&matches)
                .expect("TODO: Return correct error code");

            dd(i, o, opts)
        },
        (false, true) =>
        {
            let i = Input::<io::Stdin>::new(&matches)
                .expect("TODO: Return correct error code");
            let o = Output::<File>::new(&matches)
                .expect("TODO: Return correct error code");

            dd(i, o, opts)
        },
        (false, false) =>
        {
            let i = Input::<io::Stdin>::new(&matches)
                .expect("TODO: Return correct error code");
            let o = Output::<io::Stdout>::new(&matches)
                .expect("TODO: Return correct error code");

            dd(i, o, opts)
        },
    };

    match result
    {
        Ok((b_in, b_out)) =>
        {
            // TODO: Print output stats, unless noxfer

            RTN_SUCCESS
        },
        Err(_) =>
            RTN_FAILURE,
    }
}

