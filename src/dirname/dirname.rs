#![crate_name = "uu_dirname"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

use std::path::Path;

static NAME: &'static str = "dirname";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();
    opts.optflag("z", "zero", "separate output with NUL rather than newline");
    opts.optflag("", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!("Invalid options\n{}", f)
    };

    if matches.opt_present("help") {
        let msg = format!("{0} {1} - strip last component from file name

Usage:
  {0} [OPTION] NAME...

Output each NAME with its last non-slash component and trailing slashes
removed; if NAME contains no  /'s,  output  '.'  (meaning  the  current
directory).", NAME, VERSION);

        print!("{}", opts.usage(&msg));
        return 0;
    }

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    let separator = match matches.opt_present("zero") {
        true => "\0",
        false => "\n"
    };

    if !matches.free.is_empty() {
        for path in matches.free.iter() {
            let p = Path::new(path);
            let d = p.parent().unwrap().to_str();
            if d.is_some() {
                print!("{}", d.unwrap());
            }
            print!("{}", separator);
        }
    } else {
        println!("{0}: missing operand", NAME);
        println!("Try '{0} --help' for more information.", NAME);
        return 1;
    }

    0
}
