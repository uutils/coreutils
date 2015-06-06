#![crate_name = "join"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Gianpaolo Branca <gianpi101@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use std::io::{BufRead, BufReader};
use std::env;
use std::fs::File;
use std::cmp::Ordering;

extern crate getopts;
use getopts::Options;

//extern crate regex;
//use regex::Regex;

static VERSION: &'static str = "1.0.0";
static AUTHOR: &'static str = "Gianpaolo Branca";

/*enum CheckOrder {
    default,
    check-order,
    no-check-order,
}*/

// constructor of the output
enum Format {
    Default,
    Custom(String), // option -o given
}
// generic struct to manage variables from file1 and file2
struct Couple<F> {
    f1: F,
    f2: F,
}

impl Format {
    pub fn build(&self, s1: String, s2: String, t: char) -> String {

        let mut iters = Couple {
            f1: s1.split(t),
            f2: s2.split(t)
        };

        let mut buf = String::new();

        match *self {

            Format::Default => {

                for token in iters.f1 {
                    buf.push_str(token);
                    buf.push(t); }

                if  iters.f2.next().is_some() {

                    for token in iters.f2 {
                        buf.push_str(token);
                        buf.push(t); }
                }


            }

            Format::Custom(ref rule) => {

                let tokens = Couple {
                    f1: iters.f1.collect::<Vec<&str>>(),
                    f2: iters.f2.collect::<Vec<&str>>()
                };

                for item in rule.split(',') {

                    let item_buf = item.split('.');

    			    match item_buf.clone().count() {
                        // fields should be formatted as "X.Y" or "0"
                        1 => { if item != "0" {
                                panic!("invalid field {}", item); }

                               if tokens.f1.len() > 0 {
                                   buf.push_str(tokens.f1[0]);
                                   buf.push(t); }
                               else {
                                   buf.push_str(tokens.f2[0]);
                                   buf.push(t); }
                             },

                        2 => match item_buf.clone().nth(0).unwrap() {
                                 "1" => { match item_buf.clone().nth(1).unwrap().parse::<usize>() {
                                              Ok(f)   => {
                                                  if tokens.f1.len() >= f {
                                                     buf.push_str(tokens.f1[f-1]);
                                                     buf.push(t); };
                                                  }
                                              Err(e)  => panic!(e.to_string()), };
                                        },

                                 "2" => { match item_buf.clone().nth(1).unwrap().parse::<usize>() {
                                                Ok(f)   => {
                                                    if tokens.f2.len() >= f {
                                                       buf.push_str(tokens.f2[f-1]);
                                                       buf.push(t); };
                                                    }
                                                Err(e)  => panic!(e.to_string()), };
                                        },

                                  _  => { panic!("invalid field in {}", item) },
                        },

                        _ => { panic!("invalid file number {}", item); },
                    }
                }
            }
        }

        buf
    }
}

pub fn main() {

    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();
// to complete
    opts.optflag("",
                 "version",
                 "print the version of the program");
// to complete
    opts.optflag("h",
                 "help",
                 "print this help menu");
// waiting for implementation in standard libray.
// to_uppercase() and to_lowercase() are not yet s
    opts.optflag("i",
                 "ignore-case",
                 "ignore differences in case when comparing fields");
// to do
	opts.optflag("",
				 "header",
				 "treat the first line in each file as field headers, print them without trying to pair them");
// to do
    opts.optflag("",
                 "check-order",
                 "check that the input is correctly sorted, even if all input lines are pairable");
// to do
    opts.optflag("",
                 "nocheck-order",
                 "do not check that the input is correctly sorted");

// working
    opts.optmulti("a",
                  "all",
                  "print also non joinable lines from FILE1 or FILE2",
                  "-a FILENUM");
// working
	opts.optmulti("v",
				  "",
				  "like -a FILENUM, but suppress joined output lines",
				  "-v FILENUM");

// working
    opts.optopt("t",
                "tabulator",
                "use CHAR as an input and output filed separator",
                "-t CHAR");
// to do
	opts.optopt("e",
				"empty",
				"replace missing input fields with EMPTY",
				"-e EMPTY");
// working
	opts.optopt("o",
				"obey",
				"obey FORMAT while constructing output file",
				"-o FORMAT");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()), };
// Option help
    if matches.opt_present("h") {
        println!("coming soon!");
        return; }
// Option version
    if matches.opt_present("version") {
        println!("join {} Rust rewritten\n\nAuthor: {}", VERSION, AUTHOR);
        return; }
// Option all
    let mut flag_a = Couple { f1: false, f2: false };

    for strs in matches.opt_strs("a") {
        match strs.as_ref() {
            "1" => flag_a.f1 = true,
            "2" => flag_a.f2 = true,
             _  => { println!("invalid argument");
                     return}, } }
// Option v
	let mut flag_v: bool = false;

	for strs in matches.opt_strs("v") {
        match strs.as_ref() {
            "1" => { flag_a.f1 = true;
					 flag_v = true },

            "2" => { flag_a.f2 = true;
					 flag_v = true },

             _  => { println!("invalid argument");
                     return}, } }
// Option tabulator
    let flag_t: char = match matches.opt_str("t").as_ref() {

        Some(t) => {
            if t.len() > 1 {
                println!("invalid multichar tabulator: {}", t);
                return }
            t.clone().pop().unwrap() },

        None => ' ', };
// Option obey
	let format: Format = match matches.opt_str("o") {

		Some(f) => { Format::Custom(f) },
		None => Format::Default,
    };
// stuff begins
    let files = Couple {

        f1: match File::open(&matches.free[0]) {
            Ok(file1) => file1,
            Err(_)    => panic!("could not open {}"),
        },

        f2: match File::open(&matches.free[1]) {
            Ok(file2) => file2,
            Err(_)    => panic!("could not open {}"),
        }
    };

    let mut iters = Couple {
        f1: BufReader::new(files.f1).lines(),
        f2: BufReader::new(files.f2).lines()
    };

    let mut opts = Couple {
        f1: iters.f1.next(),
        f2: iters.f2.next()
    };

    while opts.f1.is_some() && opts.f2.is_some() {

        let strs = Couple {

            f1: opts.f1.as_ref().unwrap()
                       .as_ref().unwrap().clone(),

            f2: opts.f2.as_ref().unwrap()
                       .as_ref().unwrap().clone()
        };

        match  strs.f1.split(flag_t).nth(0).unwrap()
                      .cmp(strs.f2.split(flag_t).nth(0).unwrap()) {
            Ordering::Equal   => {

                if !flag_v { println!("{}",format.build(strs.f1, strs.f2, flag_t)) };

                opts.f1 = iters.f1.next();
                opts.f2 = iters.f2.next(); }

            Ordering::Less    => {

                if flag_a.f1 { println!("{}",format.build(strs.f1, "".to_string(), flag_t)) };
                opts.f1 = iters.f1.next(); }

            Ordering::Greater => {

                if flag_a.f2 { println!("{}",format.build("".to_string(), strs.f1, flag_t)) };
                opts.f2 = iters.f2.next(); }
		}
	}
// the following part is written because one of the files may be longer,
// and opt.is_some() may be still something
    while flag_a.f1 && opts.f1.is_some() {

        let str1 = opts.f1.as_ref().unwrap()
                       .as_ref().unwrap().clone();

        println!("{}",format.build(str1, "".to_string(), flag_t));

        opts.f1 = iters.f1.next(); }

    while flag_a.f2 && opts.f2.is_some() {

        let str2 = opts.f2.as_ref().unwrap()
                       .as_ref().unwrap().clone();

        println!("{}",format.build("".to_string(), str2, flag_t));

        opts.f2 = iters.f2.next(); }
}
