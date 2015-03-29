#![feature(core, exit_status, old_io, old_path)]
use std::env;
use std::old_io::{File, Truncate, ReadWrite, Writer};
use std::old_path::Path;

static TEMPLATE: &'static str = "\
#![feature(exit_status)]
extern crate \"@UTIL_CRATE@\" as uu@UTIL_CRATE@;

use std::env;
use uu@UTIL_CRATE@::uumain;

fn main() {
    env::set_exit_status(uumain(env::args().collect()));
}
";

fn main() {
    let args : Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("usage: mkbuild <crate> <outfile>");
        env::set_exit_status(1);
        return;
    }

    let crat    = args[1].as_slice();
    let outfile = args[2].as_slice();

    let main = TEMPLATE.replace("@UTIL_CRATE@", crat);
    let mut out = File::open_mode(&Path::new(outfile), Truncate, ReadWrite);

    if out.is_ok() {
        match out.unwrap().write_all(main.as_bytes()) {
            Err(e) => panic!("{}", e),
            _ => (),
        }
    }
}
