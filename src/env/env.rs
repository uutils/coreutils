#![crate_name = "env"]

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

use std::env;
use std::io::Write;
use std::process::Command;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "env";
static VERSION: &'static str = "1.0.0";

struct options {
    ignore_env: bool,
    null: bool,
    unsets: Vec<String>,
    sets: Vec<(String, String)>,
    program: Vec<String>,
}

fn usage() {
    println!("Usage: {} [OPTION]... [-] [NAME=VALUE]... [COMMAND [ARG]...]",
             NAME);
    println!("Set each NAME to VALUE in the environment and run COMMAND\n");
    println!("Possible options are:");
    println!("  -i --ignore-environment\t start with an empty environment");
    println!("  -0 --null              \t end each output line with a 0 byte rather than newline");
    println!("  -u --unset NAME        \t remove variable from the environment");
    println!("  -h --help              \t display this help and exit");
    println!("  -V --version           \t output version information and exit\n");
    println!("A mere - implies -i. If no COMMAND, print the resulting environment");
}

fn version() {
    println!("{} {}", NAME, VERSION);
}

// print name=value env pairs on screen
// if null is true, separate pairs with a \0, \n otherwise
fn print_env(null: bool) {
    for (n, v) in env::vars() {
        print!("{}={}{}",
               n,
               v,
               if null {
                   '\0'
               } else {
                   '\n'
               });
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    // to handle arguments the same way than GNU env, we can't use getopts
    let mut opts = Box::new(options {
        ignore_env: false,
        null: false,
        unsets: vec!(),
        sets: vec!(),
        program: vec!(),
    });

    let mut wait_cmd = false;
    let mut iter = args.iter();
    iter.next(); // skip program
    let mut item = iter.next();

    // the for loop doesn't work here,
    // because we need sometines to read 2 items forward,
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
                    opts.sets.push((n.to_string(), v.to_string()));
                }
                _ => {
                    // read the program now
                    opts.program.push(opt.to_string());
                    break;
                }
            }
        } else if opt.starts_with("--") {
            match opt.as_ref() {
                "--help" => {
                    usage();
                    return 0;
                }
                "--version" => {
                    version();
                    return 0;
                }

                "--ignore-environment" => opts.ignore_env = true,
                "--null" => opts.null = true,
                "--unset" => {
                    let var = iter.next();

                    match var {
                        None => println!("{}: this option requires an argument: {}", NAME, opt),
                        Some(s) => opts.unsets.push(s.to_string()),
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
                    'h' => {
                        usage();
                        return 0;
                    }
                    'V' => {
                        version();
                        return 0;
                    }
                    'i' => opts.ignore_env = true,
                    '0' => opts.null = true,
                    'u' => {
                        let var = iter.next();

                        match var {
                            None => println!("{}: this option requires an argument: {}", NAME, opt),
                            Some(s) => opts.unsets.push(s.to_string()),
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
                    opts.sets.push((n.to_string(), v.to_string()));
                    wait_cmd = true;
                }
                // no, its a program-like opt
                _ => {
                    opts.program.push(opt.to_string());
                    break;
                }
            }
        }

        item = iter.next();
    }

    // read program arguments
    for opt in iter {
        opts.program.push(opt.to_string());
    }

    if opts.ignore_env {
        for (ref name, _) in env::vars() {
            env::remove_var(name);
        }
    }

    for name in opts.unsets.iter() {
        env::remove_var(name);
    }

    for &(ref name, ref val) in opts.sets.iter() {
        env::set_var(name, val);
    }

    if opts.program.len() >= 1 {
        let prog = opts.program[0].clone();
        let args = &opts.program[1..];
        match Command::new(prog).args(args).status() {
            Ok(exit) => return if exit.success() {
                0
            } else {
                exit.code().unwrap()
            },
            Err(_) => return 1,
        }
    } else {
        // no program provided
        print_env(opts.null);
        pipe_flush!();
    }

    0
}
