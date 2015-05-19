use std::env;
use std::io::Write;
use std::fs::File;

static TEMPLATE: &'static str = "\
extern crate @UTIL_CRATE@ as uu@UTIL_CRATE@;

use std::env;
use uu@UTIL_CRATE@::uumain;

fn main() {
    std::process::exit(uumain(env::args().collect()));
}
";

fn main() {
    let args : Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("usage: mkbuild <crate> <outfile>");
        std::process::exit(1);
    }

    let crat    = match &args[1][..] {
        "false" => "uufalse",
        "test" => "uutest",
        "true" => "uutrue",
        _ => &args[1][..],
    };
    let outfile = &args[2][..];

    let main = TEMPLATE.replace("@UTIL_CRATE@", crat);
    match File::create(outfile) {
        Ok(mut out) => match out.write_all(main.as_bytes()) {
            Err(e) => panic!("{}", e),
            _ => (),
        },
        Err(e) => panic!("{}", e),
    }
}
