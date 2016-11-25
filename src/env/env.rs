#![crate_name = "uu_env"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: env (GNU coreutils) 8.13 */

#![allow(non_camel_case_types)]

#[macro_use]
extern crate uucore;

use std::env;
use std::io::Write;
use std::process::Command;

static NAME: &'static str = "env"; 
static SYNTAX: &'static str = "[OPTION]... [-] [NAME=VALUE]... [COMMAND [ARG]...]"; 
static SUMMARY: &'static str = "Set each NAME to VALUE in the environment and run COMMAND"; 
static LONG_HELP: &'static str = "
 A mere - implies -i. If no COMMAND, print the resulting environment
"; 

struct options {
    ignore_env: bool,
    null: bool,
    unsets: Vec<String>,
    sets: Vec<(String, String)>,
    program: Vec<String>
}

// print name=value env pairs on screen
// if null is true, separate pairs with a \0, \n otherwise
fn print_env(null: bool) {
    for (n, v) in env::vars() {
        print!("{}={}{}", n, v, if null { '\0' } else { '\n' });
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut core_opts = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP);
    core_opts.optflag("i", "ignore-environment", "start with an empty environment")
        .optflag("0", "null", "end each output line with a 0 byte rather than newline")
        .optopt("u", "unset", "remove variable from the environment", "NAME");
        
    let mut opts = Box::new(options {
        ignore_env: false,
        null: false,
        unsets: vec!(),
        sets: vec!(),
        program: vec!()
    });

    let mut wait_cmd = false;
    let mut iter = args.iter();
    iter.next(); // skip program
    let mut item = iter.next();

    // the for loop doesn't work here,
    // because we need sometimes to read 2 items forward,
    // and the iter can't be borrowed twice
    while item != None {
        let opt = item.unwrap();

        if wait_cmd {
            // we still accept NAME=VAL here but not other options
            let mut sp = opt.splitn(2, '=');
            let name = sp.next();
            let value = sp.next();

            match (name, value) {
                (Some(n), Some(v)) => {
                    opts.sets.push((n.to_owned(), v.to_owned()));
                }
                _ => {
                    // read the program now
                    opts.program.push(opt.to_owned());
                    break;
                }
            }
        } else if opt.starts_with("--") {
            match opt.as_ref() {
                "--help" => { core_opts.parse(vec![String::from("--help")]); return 0; }
                "--version" => { core_opts.parse(vec![String::from("--version")]); return 0; }

                "--ignore-environment" => opts.ignore_env = true,
                "--null" => opts.null = true,
                "--unset" => {
                    let var = iter.next();

                    match var {
                        None => println!("{}: this option requires an argument: {}", NAME, opt),
                        Some(s) => opts.unsets.push(s.to_owned())
                    }
                }

                _ => {
                    println!("{}: invalid option \"{}\"", NAME, *opt);
                    println!("Type \"{} --help\" for detailed informations", NAME);
                    return 1;
                }
            }
        } else if opt.starts_with("-") {
            if opt.len() == 1 {
                // implies -i and stop parsing opts
                wait_cmd = true;
                opts.ignore_env = true;
                continue;
            }

            let mut chars = opt.chars();
            chars.next();

            for c in chars {
                // short versions of options
                match c {
                    'i' => opts.ignore_env = true,
                    '0' => opts.null = true,
                    'u' => {
                        let var = iter.next();

                        match var {
                            None => println!("{}: this option requires an argument: {}", NAME, opt),
                            Some(s) => opts.unsets.push(s.to_owned())
                        }
                    }
                    _ => {
                        println!("{}: illegal option -- {}", NAME, c);
                        println!("Type \"{} --help\" for detailed informations", NAME);
                        return 1;
                    }
                }
            }
        } else {
            // is it a NAME=VALUE like opt ?
            let mut sp = opt.splitn(2, "=");
            let name = sp.next();
            let value = sp.next();

            match (name, value) {
                (Some(n), Some(v)) => {
                    // yes
                    opts.sets.push((n.to_owned(), v.to_owned()));
                    wait_cmd = true;
                }
                // no, its a program-like opt
                _ => {
                    opts.program.push(opt.clone());
                    break;
                }
            }
        }

        item = iter.next();
    }

    // read program arguments
    for opt in iter {
        opts.program.push(opt.clone())
    }

    if opts.ignore_env {
        for (ref name, _) in env::vars() {
            env::remove_var(name);
        }
    }

    for name in &opts.unsets {
        env::remove_var(name);
    }

    for &(ref name, ref val) in &opts.sets {
        env::set_var(name, val);
    }

    if opts.program.len() >= 1 {
        let prog = opts.program[0].clone();
        let args = &opts.program[1..];
        match Command::new(prog).args(args).status() {
            Ok(exit) => return if exit.success() { 0 } else { exit.code().unwrap() },
            Err(_) => return 1
        }
    } else {
        // no program provided
        print_env(opts.null);
        pipe_flush!();
    }

    0
}
