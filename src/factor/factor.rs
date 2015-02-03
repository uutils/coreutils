#![crate_name = "factor"]
#![feature(collections, core, io, libc, rustc_private)]

/*
* This file is part of the uutils coreutils package.
*
* (c) T. Jameson Little <t.jameson.little@gmail.com>
*
* For the full copyright and license information, please view the LICENSE file
* that was distributed with this source code.
*/

extern crate getopts;
extern crate libc;

use std::vec::Vec;
use std::old_io::BufferedReader;
use std::old_io::stdio::stdin_raw;

#[path="../common/util.rs"]
#[macro_use]
mod util;

static VERSION: &'static str = "1.0.0";
static NAME: &'static str = "factor";

fn factor(mut num: u64) -> Vec<u64> {
    let mut ret = Vec::new();

    if num < 2 {
        return ret;
    }
    while num % 2 == 0 {
        num /= 2;
        ret.push(2);
    }
    let mut i = 3;
    while i * i <= num {
        while num % i == 0 {
            num /= i;
            ret.push(i);
        }
        i += 2;
    }
    if num > 1 {
        ret.push(num);
    }
    ret
}

fn print_factors(num: u64) {
    print!("{}:", num);
    for fac in factor(num).iter() {
        print!(" {}", fac);
    }
    println!("");
}

fn print_factors_str(num_str: &str) {
    let num = match num_str.parse::<u64>() {
        Ok(x) => x,
        Err(e)=> { crash!(1, "{} not a number: {}", num_str, e); }
    };
    print_factors(num);
}

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].as_slice();
    let opts = [
        getopts::optflag("h", "help", "show this help message"),
        getopts::optflag("v", "version", "print the version and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f)
    };

    if matches.opt_present("help") {
        print!("{program} {version}\n\
                \n\
                Usage:\n\
                \t{program} [NUMBER]...\n\
                \t{program} [OPTION]\n\
                \n\
                {usage}", program = program, version = VERSION, usage = getopts::usage("Print the prime factors of the given number(s). \
                                        If none are specified, read from standard input.", &opts));
        return 1;
    }
    if matches.opt_present("version") {
        println!("{} {}", program, VERSION);
        return 0;
    }

    if matches.free.is_empty() {
        for line in BufferedReader::new(stdin_raw()).lines() {
            print_factors_str(line.unwrap().as_slice().trim());
        }
    } else {
        for num_str in matches.free.iter() {
            print_factors_str(num_str.as_slice());
        }
    }
    0
}
