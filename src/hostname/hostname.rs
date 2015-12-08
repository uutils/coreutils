#![crate_name = "uu_hostname"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alan Andrade <alan.andradec@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 *
 * Synced with:
 *
 * https://www.opensource.apple.com/source/shell_cmds/shell_cmds-170/hostname/hostname.c?txt
 */

extern crate getopts;
extern crate libc;

#[macro_use]
extern crate uucore;

use getopts::Options;
use std::collections::hash_set::HashSet;
use std::iter::repeat;
use std::str;
use std::io::Write;
use std::net::ToSocketAddrs;

static NAME: &'static str = "hostname";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

extern {
    fn gethostname(name: *mut libc::c_char, namelen: libc::size_t) -> libc::c_int;
}

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
extern {
    fn sethostname(name: *const libc::c_char, namelen: libc::c_int) -> libc::c_int;
}

#[cfg(target_os = "linux")]
extern {
    fn sethostname(name: *const libc::c_char, namelen: libc::size_t) -> libc::c_int;
}

pub fn uumain(args: Vec<String>) -> i32 {
    let program = &args[0];

    let mut opts = Options::new();
    opts.optflag("d", "domain", "Display the name of the DNS domain if possible");
    opts.optflag("i", "ip-address", "Display the network address(es) of the host");
    opts.optflag("f", "fqdn", "Display the FQDN (Fully Qualified Domain Name) (default)");   // TODO: support --long
    opts.optflag("s", "short", "Display the short hostname (the portion before the first dot) if possible");
    opts.optflag("h", "help", "Show help");
    opts.optflag("V", "version", "Show program's version");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        _ => { help_menu(program, opts); return 0; }
    };

    if matches.opt_present("h") {
        help_menu(program, opts);
        return 0
    }
    if matches.opt_present("V") { version(); return 0 }

    match matches.free.len() {
        0 => {
            let hostname = xgethostname();

            if matches.opt_present("i") {
                match hostname.to_socket_addrs() {
                    Ok(addresses) => {
                        let mut hashset = HashSet::new();
                        let mut output = String::new();
                        for addr in addresses {
                            // XXX: not sure why this is necessary...
                            if !hashset.contains(&addr) {
                                output.push_str(&format!("{}", addr));
                                output.push_str(" ");
                                hashset.insert(addr.clone());
                            }
                        }
                        let len = output.len();
                        if len > 0 {
                            println!("{}", &output[0 .. len - 1]);
                        }
                    }
                    Err(f) => {
                        show_error!("{}", f);
                        return 1;
                    }
                }
            } else {
                if matches.opt_present("s") {
                    let mut it = hostname.char_indices().filter(|&ci| ci.1 == '.');
                    let ci = it.next();
                    if ci.is_some() {
                        println!("{}", &hostname[0 .. ci.unwrap().0]);
                        return 0;
                    }
                } else if matches.opt_present("d") {
                    let mut it = hostname.char_indices().filter(|&ci| ci.1 == '.');
                    let ci = it.next();
                    if ci.is_some() {
                        println!("{}", &hostname[ci.unwrap().0 + 1 .. ]);
                        return 0;
                    }
                }

                println!("{}", hostname);
            }
        }
        1 => xsethostname(matches.free.last().unwrap()),
        _ => help_menu(program, opts)
    };

    0
}

fn version() {
    println!("{} {}", NAME, VERSION);
}

fn help_menu(program: &str, options: Options) {
    version();
    println!("");
    println!("Usage:");
    println!("  {} [OPTION]... [HOSTNAME]", program);
    println!("");
    print!("{}", options.usage("Print or set the system's host name."));
}

fn xgethostname() -> String {
    let namelen = 256usize;
    let mut name : Vec<u8> = repeat(0).take(namelen).collect();
    let err = unsafe {
        gethostname (name.as_mut_ptr() as *mut libc::c_char,
                                        namelen as libc::size_t)
    };

    if err != 0 {
        panic!("Cannot determine hostname");
    }

    let last_char = name.iter().position(|byte| *byte == 0).unwrap_or(namelen);

    str::from_utf8(&name[..last_char]).unwrap().to_string()
}

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
fn xsethostname(name: &str) {
    let vec_name: Vec<libc::c_char> = name.bytes().map(|c| c as libc::c_char).collect();

    let err = unsafe {
        sethostname (vec_name.as_ptr(), vec_name.len() as libc::c_int)
    };

    if err != 0 {
        println!("Cannot set hostname to {}", name);
    }
}

#[cfg(target_os = "linux")]
fn xsethostname(name: &str) {
    let vec_name: Vec<libc::c_char> = name.bytes().map(|c| c as libc::c_char).collect();

    let err = unsafe {
        sethostname (vec_name.as_ptr(), vec_name.len() as libc::size_t)
    };

    if err != 0 {
        println!("Cannot set hostname to {}", name);
    }
}
