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

extern crate regex;
use regex::Regex;

static VERSION: &'static str = "1.0.0";
static AUTHOR: &'static str = "Gianpaolo Branca";

/*enum CheckOrder {
    default,
    check-order,
    no-check-order,
}*/
#[derive(Clone)]
enum Field {
    Index,
    File1(usize),
    File2(usize),
}

// constructor of the output
enum Format {
    Default,
    Custom(Vec<Field>), // option -o given
}

// generic struct to manage variables from file1 and file2
#[derive(Clone)]
struct Couple<F> {
    f1: F,
    f2: F,
}

impl Format {

    pub fn build<'a>(&self,
                 mut line_tok1: Vec<&'a str>,
                 mut line_tok2: Vec<&'a str>,
                 t: char,
                 e: Option<String>,
                 i: Couple<usize>)     -> String {

        let mut buf = String::new();
        match *self {

            Format::Default => {

                if i.f1 != 1 {
                    let b = line_tok1.remove(i.f1-1);
                    line_tok1.insert(0, b);
                }

                if i.f2 != 1 {
                    let b = line_tok2.remove(i.f2-1);
                    line_tok2.insert(0, b);
                }


                if line_tok1.first().is_some() {
                    buf.push_str(line_tok1.first().as_ref().unwrap());
                } else {
                    buf.push_str(line_tok2.first().as_ref().unwrap());
                };

                if line_tok1.len() > 1 {
                    for token in line_tok1.into_iter().skip(1) {
                        buf.push(t);
                        buf.push_str(token);
                    }
                } else if e.is_some() {
                    buf.push(t);
                    buf.push_str(&e.clone().unwrap());
                }

                if line_tok2.len() > 1 {
                    for token in line_tok2.into_iter().skip(1) {
                        buf.push(t);
                        buf.push_str(token);
                    }
                } else if e.is_some() {
                    buf.push(t);
                    buf.push_str(&e.clone().unwrap());
                }

            }

            Format::Custom(ref rule) => {
                // implementation for -j,-1,-2 still required
                for field in rule.clone() {
                    match field {

                        Field::Index => {
                            if line_tok1.len() > 0 {
                                buf.push_str(line_tok1[0]);
                                buf.push(t); }
                            else {
                                buf.push_str(line_tok2[0]);
                                buf.push(t); }
                        },

                        Field::File1(n) => {
                            if line_tok1.len() > n {
                                buf.push_str(line_tok1[n]);
                                buf.push(t);
                            } else if e.is_some() {
                               buf.push_str(&e.clone().unwrap());
                               buf.push(t);
                            }
                        },

                        Field::File2(n) => {
                            if line_tok2.len() > n {
                                buf.push_str(line_tok2[n]);
                                buf.push(t);
                            } else if e.is_some() {
                               buf.push_str(&e.clone().unwrap());
                               buf.push(t);
                            }
                        },
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
// working
    opts.optflag("i",
                 "ignore-case",
                 "ignore differences in case when comparing fields");
// working
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
// working
	opts.optopt("e",
				"empty",
				"replace missing input fields with EMPTY",
				"-e EMPTY");
// working
	opts.optopt("o",
				"obey",
				"obey FORMAT while constructing output file",
				"-o FORMAT");
// working?
    opts.optopt("j",
                "",
                "equivalent to '-1 FIELD -2 FIELD'",
                "-j FIELD");
// working?
    opts.optopt("1",
                "",
                "join on this FIELD of file 1",
                "-1 FIELD");
// working?
    opts.optopt("2",
                "",
                "join on this FIELD of file 2",
                "-2 FIELD");

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
                     return}, } };
// Option tabulator
    let flag_t = match matches.opt_str("t").as_ref() {

        Some(t) => {
            if t.len() > 1 {
                println!("invalid multichar tabulator: {}", t);
                return }
            t.clone().pop().unwrap() },

        None => ' ', };
// Option obey
	let format: Format = match matches.opt_str("o") {
		None => Format::Default,
        Some(rule) => {

            let mut buf: Vec<Field> = Vec::new();
            for item in rule.split(',') {

                let sub_item = item.split('.');
                match sub_item.clone().count() {

                    1 => {
                        if item != "0" {
                            panic!("invalid field {}", item); }
                        buf.push(Field::Index);
                    },

                    2 => {

                        match sub_item.clone().nth(0).unwrap() {

                            "1" => {
                                match sub_item.clone().nth(1).unwrap().parse::<usize>() {

                                    Ok(f) => buf.push(Field::File1(f-1)),
                                    Err(e) => panic!(e.to_string()),
                                };
                            },

                            "2" => {
                                match sub_item.clone().nth(1).unwrap().parse::<usize>() {

                                    Ok(f) => buf.push(Field::File2(f-1)),
                                    Err(e) => panic!(e.to_string()),
                                };
                            },

                            _  => {

                                panic!("invalid field in {}", item)
                            },
                        };
                    },

                    _ => panic!("invalid file number {}", item),

                }
            };
            // buf returned
            Format::Custom(buf)        }
    };

// Option empty
    let opt_e = matches.opt_str("e");
// Option -1, -2, j
    let mut index_pos = Couple {

        f1: match matches.opt_str("1") {

            None    => 0, // 0 means "not yet initialized"

            Some(i) => {

                if i == "0" {
                    println!("invalid field: 0");
                    return
                } else {
                    match i.parse::<usize>() {
                        Ok(u)   => u,
                        Err(e)  => panic!(e.to_string()),
                    }
                }
            }
        },

        f2: match matches.opt_str("2") {

            None    => 0,

            Some(i) => {
                if i == "0" {
                    println!("invalid field: 0");
                    return
                } else {
                    match i.parse::<usize>() {
                        Ok(u)   => u,
                        Err(e)  => panic!(e.to_string()),
                    }
                }
            }
        }
    };

    if matches.opt_str("j").is_some() {

        let t =  match matches.opt_str("j").unwrap().parse::<usize>() {
            Ok(u)   => u,
            Err(e)  => panic!(e.to_string()),
        };

        if t == 0 {
            println!("invalid field: 0");
            return
        }

        if index_pos.f1 == 0 {
            index_pos.f1 = t;
        } else if index_pos.f1 != t {
            println!("non compatible fields: {}, {}", index_pos.f1, t);
            return
        }

        if index_pos.f2 == 0 {
            index_pos.f2 = t;
        } else if index_pos.f1 != t {
            println!("non compatible fields: {}, {}", index_pos.f1, t);
            return
        }
    }

    if index_pos.f1 == 0 {
        index_pos.f1 = 1;
    }

    if index_pos.f2 == 0 {
        index_pos.f2 = 1;
    }
// Option header
let mut flag_h: bool = matches.opt_present("header");
// Option ignore-case
let flag_i: bool = matches.opt_present("i");
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

    let mut lines = Couple {
        f1: Vec::new(),
        f2: Vec::new()
    };

    let mut to_print = Couple {
        f1: Vec::new(),
        f2: Vec::new()
    };

    for item in BufReader::new(files.f1).lines() {
        if let Ok(line) = item {
            lines.f1.push(line);
            to_print.f1.push(true);
        }
    }

    for item in BufReader::new(files.f2).lines() {
        if let Ok(line) = item {
            lines.f2.push(line);
            to_print.f2.push(true);
        }
    }

    /*let mut to_print = Couple {
        f1: Vec::with_capacity(lines.f1.len()),
        f2: Vec::with_capacity(lines.f2.len())
    };

    for i in 0..lines.f1.len() {
        to_print.f1[i] = true;
    }

    for i in 0..lines.f2.len() {
        to_print.f2[i] = true;
    }*/

    let mut checked = 0;

    for (i1, str1) in lines.f1.iter().enumerate() {

        let aux_check = checked;

        for (i2, str2) in lines.f2.iter().skip(checked).enumerate() {

            let tokens = if flag_t != ' ' {
                Couple {
                    f1: str1.split(flag_t).collect::<Vec<&str>>(),
                    f2: str2.split(flag_t).collect::<Vec<&str>>()
                }
            } else {
                let re = Regex::new(r"[ \t]+").unwrap();
                Couple {
                    f1: re.split(&str1).collect::<Vec<&str>>(),
                    f2: re.split(&str2).collect::<Vec<&str>>()
                }
            };

            if flag_h {
                println!("{}",format.build(tokens.f1, tokens.f2, flag_t, opt_e.clone(), index_pos.clone()));
                flag_h = false;
                checked += 1;
                break;
            }

            let indexes = if flag_i {
                Couple {
                    f1: tokens.f1[index_pos.f1-1].to_lowercase(),
                    f2: tokens.f2[index_pos.f2-1].to_lowercase()
                }
            } else {
                Couple {
                    f1: tokens.f1[index_pos.f1-1].to_string(),
                    f2: tokens.f2[index_pos.f2-1].to_string()
                }
            };
            match indexes.f1.cmp(&indexes.f2) {
                Ordering::Equal   => {
                    if !flag_v {
                        println!("{}",format.build(tokens.f1, tokens.f2, flag_t, opt_e.clone(), index_pos.clone()));
                    };
                    to_print.f1[i1] = false;
                    to_print.f2[i2 + aux_check] = false;
                }

                Ordering::Less    => {

                    if flag_a.f1 && to_print.f1[i1] {
                        println!("{}",format.build(tokens.f1, Vec::new(), flag_t, opt_e.clone(), index_pos.clone()));
                        to_print.f1[i1] = false;
                    }
                    break
                }

                Ordering::Greater => {

                    if flag_a.f2 && to_print.f2[i2 + aux_check] {
                        println!("{}",format.build(Vec::new(), tokens.f2, flag_t, opt_e.clone(), index_pos.clone()));
                        to_print.f2[i2 + aux_check] = false;
                    }
                    checked += 1;
                    continue
                }
            }
        }
    }

    if flag_a.f1 {
        for (i1,str1) in lines.f1.iter().enumerate() {

            if to_print.f1[i1] == false {
                continue
            }

            if str1.is_empty() {
                continue
            }

            let tokens = if flag_t != ' ' {
                str1.split(flag_t).collect::<Vec<&str>>()
            } else {
                let re = Regex::new(r"[ \t]+").unwrap();
                re.split(&str1).collect::<Vec<&str>>()
            };

            println!("{}",format.build(tokens, Vec::new(), flag_t, opt_e.clone(), index_pos.clone()));

        }
    }

    if flag_a.f2 {
        for (i2,str2) in lines.f2.iter().enumerate() {

            if to_print.f2[i2] == false {
                continue
            }

            if str2.is_empty() {
                continue
            }

            let tokens = if flag_t != ' ' {
                str2.split(flag_t).collect::<Vec<&str>>()
            } else {
                let re = Regex::new(r"[ \t]+").unwrap();
                re.split(&str2).collect::<Vec<&str>>()
            };
            println!("{}",format.build(Vec::new(), tokens, flag_t, opt_e.clone(), index_pos.clone()));
        }
    }
}
