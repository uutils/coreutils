// This file is part of the uutils coreutils package.
//
// (c) Tyler Steele <tyler.steele@protonmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (T0DO)

#[macro_use]
extern crate uucore;

use std::error::Error;
use std::fs::File;
use std::io::{
    self, Read, Write,
    BufWriter,
};
use std::sync::mpsc;
use std::thread;

const NAME: &str = "dd";
const SUMMARY: &str = "Copies, and optionally converts, file system resources.";
const LONG_HELP: &str = "
TODO: This is where the long help string for dd goes!
";

const RTN_SUCCESS: i32 = 0;
const RTN_FAILURE: i32 = 1;

// Conversion tables are just lookup tables.
// eg.
// The ASCII->EBDIC table stores the EBDIC code at the index
// obtained by treating the ASCII representation as a number.
// This idea is from the original GNU implementation.
type ConversionTable = [u8; u8::MAX as usize];

struct Input<R: Read>
{
    src: R,
    read_size: usize,
}

impl<R: Read> Read for Input<R>
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>
    {
        self.src.read(buf)
    }
}

struct Output<W: Write>
{
    dst: W,
    write_size: usize,
    conv_table: Option<ConversionTable>,
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



fn dd<R: Read, W: Write>(mut i: Input<R>, mut o: Output<W>) -> Result<(), Box<dyn Error>>
{
    let (prog_tx, prog_rx) = mpsc::channel();

    thread::spawn(move || {
        // TODO: Replace ?? with accurate info
        print!("Progress ({}/??)", 0);

        loop {
            let prog = prog_rx.recv()
                   .expect("TODO: Handle this error in the project-specific way");
            print!("\rProgress ({}/??)", prog);
        }
    });

    let mut buf = vec![0; i.read_size];

    loop
    {
        let r_len = i.read(&mut buf)?;
        if r_len == 0 { break; }

        let w_len = o.write(&buf[..r_len])?;

        // if *full write buffer* { o.flush(); }

        prog_tx.send(w_len)?;

        buf.clear();
    }

    Ok(())
}

pub fn uumain(args: impl uucore::Args) -> i32
{
    // TODO: parse args

    let if_name = "foo.txt";
    let of_name = "bar.txt";
    let read_size = 512;
    let write_size = 4096;

    let in_f = File::open(if_name)
        .expect("TODO: Handle this error in the project-specific way");

    let out_f = File::open(of_name)
        .expect("TODO: Handle this error in the project-specific way");
    let out_f = BufWriter::with_capacity(write_size, out_f);

    let i = Input {
        src: in_f,
        read_size,
    };
    let o = Output {
        dst: out_f,
        write_size,
        conv_table: None,
    };

    match dd(i, o) {
        Ok(_) =>
            RTN_SUCCESS,
        Err(_) =>
            RTN_FAILURE,
    }
}

#[cfg(test)]
mod test_dd_internal
{
    #[allow(unused_imports)]
    use super::*;

    use std::io::prelude::*;

    #[test]
    fn empty_reader_test()
    {
        let src = io::empty();
       
        let dst = vec![0xFF as u8, 128];
        let dst_ptr = dst.as_ptr();
        let exp = vec![0xFF as u8, 128];

        let i = Input {
            src,
            read_size: 1,
        };

        let o = Output {
            dst,
            write_size: 1,
            conv_table: None,
        };

        dd(i,o).unwrap();

        for (i, byte) in exp.iter().enumerate()
        {
            panic!();
        }
    }
}
