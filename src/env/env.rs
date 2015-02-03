#![crate_name = "env"]
#![feature(core, io, os)]

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
#![feature(box_syntax)]

struct options {
    ignore_env: bool,
    null: bool,
    unsets: Vec<String>,
    sets: Vec<(String, String)>,
    program: Vec<String>
}

fn usage(prog: &str) {
    println!("Usage: {} [OPTION]... [-] [NAME=VALUE]... [COMMAND [ARG]...]", prog);
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
    println!("env 1.0.0");
}

// print name=value env pairs on screen
// if null is true, separate pairs with a \0, \n otherwise
fn print_env(null: bool) {
    let env = std::os::env();

    for &(ref n, ref v) in env.iter() {
        print!("{}={}{}",
            n.as_slice(),
            v.as_slice(),
            if null { '\0' } else { '\n' }
        );
    }
}

pub fn uumain(args: Vec<String>) -> isize {
    let prog = args[0].as_slice();

    // to handle arguments the same way than GNU env, we can't use getopts
    let mut opts = box options {
        ignore_env: false,
        null: false,
        unsets: vec!(),
        sets: vec!(),
        program: vec!()
    };

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
            let mut sp = opt.as_slice().splitn(1, '=');
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
        } else if opt.as_slice().starts_with("--") {
            match opt.as_slice() {
                "--help" => { usage(prog); return 0; }
                "--version" => { version(); return 0; }

                "--ignore-environment" => opts.ignore_env = true,
                "--null" => opts.null = true,
                "--unset" => {
                    let var = iter.next();

                    match var {
                        None => println!("{}: this option requires an argument: {}", prog, opt.as_slice()),
                        Some(s) => opts.unsets.push(s.to_string())
                    }
                }

                _ => {
                    println!("{}: invalid option \"{}\"", prog, *opt);
                    println!("Type \"{} --help\" for detailed informations", prog);
                    return 1;
                }
            }
        } else if opt.as_slice().starts_with("-") {
            if opt.len() == 0 {
                // implies -i and stop parsing opts
                wait_cmd = true;
                opts.ignore_env = true;
                continue;
            }

            let mut chars = opt.as_slice().chars();
            chars.next();

            for c in chars {
                // short versions of options
                match c {
                    'h' => { usage(prog); return 0; }
                    'V' => { version(); return 0; }
                    'i' => opts.ignore_env = true,
                    '0' => opts.null = true,
                    'u' => {
                        let var = iter.next();

                        match var {
                            None => println!("{}: this option requires an argument: {}", prog, opt.as_slice()),
                            Some(s) => opts.unsets.push(s.to_string())
                        }
                    }
                    _ => {
                        println!("{}: illegal option -- {}", prog, c);
                        println!("Type \"{} --help\" for detailed informations", prog);
                        return 1;
                    }
                }
            }
        } else {
            // is it a NAME=VALUE like opt ?
            let mut sp = opt.as_slice().splitn(1, '=');
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

    let env = std::os::env();

    if opts.ignore_env {
        for &(ref name, _) in env.iter() {
            std::os::unsetenv(name.as_slice())
        }
    }

    for name in opts.unsets.iter() {
        std::os::unsetenv((name).as_slice())
    }

    for &(ref name, ref val) in opts.sets.iter() {
        std::os::setenv(name.as_slice(), val.as_slice())
    }

    if opts.program.len() >= 1 {
        use std::old_io::process::{Command, InheritFd};
        let prog = opts.program[0].clone();
        let args = &opts.program[1..];
        match Command::new(prog).args(args).stdin(InheritFd(0)).stdout(InheritFd(1)).stderr(InheritFd(2)).status() {
            Ok(exit) =>
                return match exit {
                    std::old_io::process::ExitStatus(s) => s,
                    _ => 1
                },
            Err(_) => return 1
        }
    } else {
        // no program provided
        print_env(opts.null);
    }

    0
}
