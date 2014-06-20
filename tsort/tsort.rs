#![crate_id(name="tsort", vers="1.0.0", author="Ben Eggers")]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Ben Eggers <ben.eggers36@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![feature(macro_rules)]

extern crate getopts;
extern crate libc;

use std::os;

use std::io::{print, stdin, File, BufferedReader};
use StdResult = std::result::Result;

#[allow(dead_code)]
fn main() { os::set_exit_status(uumain(os::args())); }

fn uumain(args: Vec<String>) -> int {
	let prog_name = args.get(0).clone();
	let opts = [
		getopts::optflag("d", "debug", "print out information as the sort happens"),
		getopts::optflag("h", "help", "display this help and exit"),
	];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(_) => {
            usage(prog_name, opts);
            return 1;
        }
    };

    if matches.opt_present("help") {
    	usage(prog_name, opts);
    	return 0;
    }

    let mut files = matches.free.clone();
    if files.is_empty() {
        files = vec!("-".to_string());
    } else if files.len() > 1 {
    	println!("{}: extra operand '{}'", prog_name, files.get(1));
    	return 1;
    }

    let mut reader = match open(files.get(0).to_string()) {
        Ok(f) => f,
        Err(_) => { return 1; }
    };


	return 0
}

// Print out the program usage
fn usage(prog_name: String, opts: [getopts::OptGroup, ..2]) {
    println!("Usage:");
	println!("	{} [OPTIONS] FILE", prog_name);
	print!("Topological sort the strings in FILE. ");
	print!("Strings are defined as any sequence of tokens separated by whitespace ");
	print!("(tab, space, or newline). If FILE is not passed in, stdin is used instead.");
	print(getopts::usage("", opts).as_slice());
	println!("");
}

fn open(path: String) -> StdResult<BufferedReader<Box<Reader>>, int> {
    if  path.as_slice == "-" {
        let reader = box stdin() as Box<Reader>;
        return Ok(BufferedReader::new(reader));
    }

    match File::open(&std::path::Path::new(path.as_slice())) {
        Ok(fd) => {
            let reader = box fd as Box<Reader>;
            Ok(BufferedReader::new(reader))
        },
        Err(_) => {
            Err(1)
        }
    }
}
