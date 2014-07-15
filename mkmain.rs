use std::io::{File, Truncate, ReadWrite};
use std::os;
use std::path::Path;
use std::str::replace;

static TEMPLATE : &'static str = r"
extern crate @UTIL_CRATE@;

use std::os;
use @UTIL_CRATE@::uumain;

fn main() {
    os::set_exit_status(uumain(os::args()));
}
";

fn main() {
    let args = os::args();
    if args.len() != 3 {
        println!("usage: mkbuild <crate> <outfile>");
        os::set_exit_status(1);
        return;
    }

    let crat = match args.get(1).as_slice() {
        "test" => "uutest",
        "true" => "uutrue",
        "false" => "uufalse",
        "sync" => "uusync",
        s => s.clone(),
    };
    let outfile  = args.get(2).as_slice();

    let main = std::str::replace(TEMPLATE, "@UTIL_CRATE@", crat);
    let mut out = File::open_mode(&Path::new(outfile), Truncate, ReadWrite);

    match out.write(main.as_bytes()) {
        Err(e) => fail!("{}", e),
        _ => (),
    }
}
