#[crate_id(name = "tac", vers = "1.0.0", author = "Arcterus")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[feature(macro_rules)];

extern crate getopts;

use std::io;
use std::os;

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "tac";
static VERSION: &'static str = "1.0.0";

fn main() {
    let args = os::args();
    let program = args[0].clone();

    let opts = ~[
        getopts::optflag("b", "before", "attach the separator before instead of after"),
        getopts::optflag("r", "regex", "interpret the sequence as a regular expression (NOT IMPLEMENTED)"),
        getopts::optopt("s", "separator", "use STRING as the separator instead of newline", "STRING"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];
    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f.to_err_msg())
    };
    if matches.opt_present("help") {
        println!("tac {}", VERSION);
        println!("");
        println!("Usage:");
        println!("  {0:s} [OPTION]... [FILE]...", program);
        println!("");
        print!("{}", getopts::usage("Write each file to standard output, last line first.", opts));
    } else if matches.opt_present("version") {
        println!("tac {}", VERSION);
    } else {
        let before = matches.opt_present("b");
        let regex = matches.opt_present("r");
        let separator = match matches.opt_str("s") {
            Some(m) => {
                if m.len() == 0 {
                    crash!(1, "separator cannot be empty")
                } else {
                    m
                }
            }
            None => ~"\n"
        };
        tac(matches.free, before, regex, separator);
    }
}

fn tac(filenames: Vec<~str>, before: bool, _: bool, separator: ~str) {
    for filename in filenames.move_iter() {
        let mut file = io::BufferedReader::new(
                           crash_if_err!(1, io::File::open(&Path::new(filename))));
        let mut data = crash_if_err!(1, file.read_to_str());
        if data.ends_with("\n") {
            data.pop_char();  // removes blank line that is inserted otherwise
        }
        let split_vec: ~[&str] = data.split_str(separator).collect();
        let rev: ~str = split_vec.rev_iter().fold(~"", |a, &b|
            a + if before {
                separator + b
            } else {
                b + separator
            }
        );
        print!("{}", rev);
    }
}
