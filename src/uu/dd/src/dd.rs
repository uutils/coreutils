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

enum SrcStat
{
    Read(usize),
    EOF,
}

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

fn gen_prog_updater(rx: mpsc::Receiver<usize>) -> impl Fn() -> ()
{
    move || { // LAAAAMBDA!

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

fn dd<R: Read, W: Write>(mut i: Input<R>, mut o: Output<W>) -> Result<(usize, usize), Box<dyn Error>>
{
    let (prog_tx, prog_rx) = mpsc::channel();
    thread::spawn(gen_prog_updater(prog_rx));

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
        o.flush()?;

        bytes_out += w_len;

        prog_tx.send(bytes_out)?;
    }

    Ok((bytes_in, bytes_out))
}

pub fn uumain(args: impl uucore::Args) -> i32
{
    // TODO: parse args

    let if_name = "foo.txt";
    let of_name = "bar.txt";
    let ibs = 512;
    let obs = 4096;

    let in_f = File::open(if_name)
        .expect("TODO: Handle this error in the project-specific way");

    let out_f = File::open(of_name)
        .expect("TODO: Handle this error in the project-specific way");
    let out_f = BufWriter::with_capacity(obs, out_f);

    let i = Input {
        src: in_f,
        ibs,
    };
    let o = Output {
        dst: out_f,
        obs,
        conv_table: None,
    };

    match dd(i, o) {
        Ok((b_in, b_out)) =>
        {
            println!("Completed: Bytes in: {}, Bytes out: {}", b_in, b_out);
           
            RTN_SUCCESS
        },
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
    use std::io::BufReader;
    use std::fs;
    use md5::{ Md5, Digest, };
    use hex_literal::hex;

    macro_rules! make_test (
        ( $test_id:ident, $test_name:expr, $src:expr, $exp:expr ) =>
        {
            #[test]
            fn $test_id()
            {
                // let test_name = "6ae59e64850377ee5470c854761551ea-ones";
                let tmp_fname = format!("./test-resources/FAILED-{}.test", $test_name);

                let i = Input {
                    src: $src,
                    ibs: 256,
                };

                let o = Output {
                    dst: File::create(&tmp_fname).unwrap(),
                    obs: 1024,
                    conv_table: None,
                };

                dd(i,o).unwrap();

                let res = {
                    let res = File::open(&tmp_fname).unwrap();
                    let res = BufReader::new(res);

                    let mut h = Md5::new();
                    for b in res.bytes()
                    {
                        h.update([b.unwrap()]);
                    }

                    h.finalize()
                };

                assert_eq!(hex!($exp), res[..]);

                fs::remove_file(&tmp_fname).unwrap();
            }
        };
    );

    make_test!(
        empty_file_test,
        "stdio-empty-file",
        io::empty(),
        "d41d8cd98f00b204e9800998ecf8427e"
    );

    make_test!(
        zeros_4k_test,
        "zeros-4k",
        File::open("./test-resources/620f0b67a91f7f74151bc5be745b7110-zeros.test").unwrap(),
        "620f0b67a91f7f74151bc5be745b7110"
    );

    make_test!(
        ones_4k_test,
        "ones-4k",
        File::open("./test-resources/6ae59e64850377ee5470c854761551ea-ones.test").unwrap(),
        "6ae59e64850377ee5470c854761551ea"
    );

    make_test!(
        deadbeef_32k_test,
        "deadbeef_32k",
        File::open("./test-resources/18d99661a1de1fc9af21b0ec2cd67ba3-deadbeef.test").unwrap(),
        "18d99661a1de1fc9af21b0ec2cd67ba3"
    );

    make_test!(
        random_73k_test,
        "random_73k",
        File::open("./test-resources/5828891cb1230748e146f34223bbd3b5-random.test").unwrap(),
        "5828891cb1230748e146f34223bbd3b5"
    );

}
