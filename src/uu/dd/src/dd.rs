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
use std::io;
use std::sync::mpsc;
use std::thread;

const NAME: &str = "dd";
const SUMMARY: &str = "Copies, and optionally converts, file system resources.";
const LONG_HELP: &str = "
TODO: This is where the long help string for dd goes!
";

struct Input
{
    src: Box<dyn io::Read>,
}

impl Input
{
    fn run(&self, tx: mpsc::SyncSender<Vec<u8>>) -> Result<(), Box<dyn Error>>
    {
        let buff = vec![0; 128];
        let msg = self.src.read(&buff)?;

        println!("Sender sending:\t\t{:?}", &msg);
        tx.send(msg)?;

        Ok(())
    }
}

struct Output
{
    dst: Box<dyn io::Write>,
}

impl Output
{
    fn run(&self, rx: mpsc::Receiver<Vec<u8>>) -> Result<(), Box<dyn Error>>
    {
        let data = rx.recv()?;

        println!("Receiver received:\t{:?}", data);

        Ok(())
    }
}

fn dd(i: Input, o: Output) -> ()
{
    let (tx, rx) = mpsc::sync_channel(0); // each send will block until the recv'r is ready for it

    thread::spawn(move || {
        i.run(tx);
    });

    thread::spawn(move || {
        o.run(rx);
    });

    loop{};
}

pub fn uumain(args: impl uucore::Args) -> i32
{
    // TODO: parse args

    -1
}

#[cfg(test)]
mod test_dd_internal {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn hello_world_test()
    {
        let (src, dst) = mpsc::channel();
        let data = vec![0xFF; 128];

        for c in data {
            src.send(c);
        }

        let i = Input {
            src: Box::new(src),
        };

        let o = Output {
            dst: Box::new(dst),
        };

        dd(i,o);

        assert_eq!(data, dst);
    }
}
