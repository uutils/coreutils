#[crate_id(name="dirname", vers="1.0.0", author="Derek Chiang")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern mod extra;

use std::os;
use std::io::{print, stderr};
use extra::getopts::groups;

static VERSION: &'static str = "1.0.0";

fn main() {
    let args = os::args();
    let program = args[0].clone();
    let opts = ~[
        groups::optflag("z", "zero", "separate output with NUL rather than newline"),
        groups::optflag("", "help", "display this help and exit"),
        groups::optflag("", "version", "output version information and exit"),
    ];

    let matches = match groups::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            writeln!(&mut stderr() as &mut Writer,
                "Invalid options\n{}", f.to_err_msg());
            os::set_exit_status(1);
            return
        }  
    };

    if matches.opt_present("help") {
        println!("dirname {:s} - strip last component from file name", VERSION);
        println!("");
        println!("Usage:");
        println!("  {0:s} [OPTION] NAME...", program);
        println!("");
        print(groups::usage("Output each NAME with its last non-slash component and trailing slashes
removed; if NAME contains no  /'s,  output  '.'  (meaning  the  current
directory).", opts));
        return;
    }

    if matches.opt_present("version") {
        return println!("dirname version: {:s}", VERSION);
    }

    let separator = match matches.opt_present("zero") {
        true => "\0",
        false => "\n"
    };

    if !matches.free.is_empty() {
        for path in matches.free.iter() {
            let p = std::path::Path::new(path.clone());
            print(std::str::from_utf8(p.dirname()));
            print(separator);
        }
    } else {
        println!("{0:s}: missing operand", program);
        println!("Try '{0:s} --help' for more information.", program);
    }
}
