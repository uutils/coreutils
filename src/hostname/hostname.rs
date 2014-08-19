#![crate_name = "hostname"]
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

#![feature(macro_rules)]

extern crate getopts;
extern crate libc;

use std::collections::hashmap::HashSet;
use std::io::net::addrinfo;
use std::str;
use getopts::{optflag, getopts, usage};

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "hostname";

extern {
    fn gethostname(name: *mut libc::c_char, namelen: libc::size_t) -> libc::c_int;
}

#[cfg(target_os = "macos")]
extern {
    fn sethostname(name: *const libc::c_char, namelen: libc::c_int) -> libc::c_int;
}

#[cfg(target_os = "linux")]
extern {
    fn sethostname(name: *const libc::c_char, namelen: libc::size_t) -> libc::c_int;
}

pub fn uumain(args: Vec<String>) -> int {
    let program = &args[0];

    let options = [
        optflag("d", "domain", "Display the name of the DNS domain if possible"),
        optflag("i", "ip-address", "Display the network address(es) of the host"),
        optflag("f", "fqdn", "Display the FQDN (Fully Qualified Domain Name) (default)"),   // TODO: support --long
        optflag("s", "short", "Display the short hostname (the portion before the first dot) if possible"),
        optflag("h", "help", "Show help"),
        optflag("V", "version", "Show program's version")
    ];

    let matches = match getopts(args.tail(), options) {
        Ok(m) => { m }
        _ => { help_menu(program.as_slice(), options); return 0; }
    };

    if matches.opt_present("h") {
        help_menu(program.as_slice(), options);
        return 0
    }
    if matches.opt_present("V") { version(); return 0 }

    match matches.free.len() {
        0 => {
            let hostname = xgethostname();

            if matches.opt_present("i") {
                match addrinfo::get_host_addresses(hostname.as_slice()) {
                    Ok(addresses) => {
                        let mut hashset = HashSet::new();
                        let mut output = String::new();
                        for addr in addresses.iter() {
                            // XXX: not sure why this is necessary...
                            if !hashset.contains(addr) {
                                output.push_str(addr.to_string().as_slice());
                                output.push_str(" ");
                                hashset.insert(addr.clone());
                            }
                        }
                        let len = output.len();
                        if len > 0 {
                            println!("{}", output.as_slice().slice_to(len - 1));
                        }
                    }
                    Err(f) => {
                        show_error!("{}", f);
                        return 1;
                    }
                }
            } else {
                if matches.opt_present("s") {
                    let pos = hostname.as_slice().find_str(".");
                    if pos.is_some() {
                        println!("{:s}", hostname.as_slice().slice_to(pos.unwrap()));
                        return 0;
                    }
                } else if matches.opt_present("d") {
                    let pos = hostname.as_slice().find_str(".");
                    if pos.is_some() {
                        println!("{}", hostname.as_slice().slice_from(pos.unwrap() + 1));
                        return 0;
                    }
                }

                println!("{:s}", hostname);
            }
        }
        1 => xsethostname(matches.free.last().unwrap().as_slice()),
        _ => help_menu(program.as_slice(), options)
    };

    0
}

fn version() {
    println!("hostname 1.0.0");
}

fn help_menu(program: &str, options: &[getopts::OptGroup]) {
    version();
    println!("");
    println!("Usage:");
    println!("  {:s} [OPTION]... [HOSTNAME]", program);
    println!("");
    print!("{:s}", usage("Print or set the system's host name.", options));
}

fn xgethostname() -> String {
    let namelen = 256u;
    let mut name = Vec::from_elem(namelen, 0u8);

    let err = unsafe {
        gethostname (name.as_mut_ptr() as *mut libc::c_char,
                                        namelen as libc::size_t)
    };

    if err != 0 {
        fail!("Cannot determine hostname");
    }

    let last_char = name.iter().position(|byte| *byte == 0).unwrap_or(namelen);

    str::from_utf8(name.slice_to(last_char)).unwrap().to_string()
}

#[cfg(target_os = "macos")]
fn xsethostname(name: &str) {
    let vec_name: Vec<libc::c_char> = name.bytes().map(|c| c as libc::c_char).collect();

    let err = unsafe {
        sethostname (vec_name.as_ptr(), vec_name.len() as libc::c_int)
    };

    if err != 0 {
        println!("Cannot set hostname to {:s}", name);
    }
}

#[cfg(target_os = "linux")]
fn xsethostname(name: &str) {
    let vec_name: Vec<libc::c_char> = name.bytes().map(|c| c as libc::c_char).collect();

    let err = unsafe {
        sethostname (vec_name.as_ptr(), vec_name.len() as libc::size_t)
    };

    if err != 0 {
        println!("Cannot set hostname to {:s}", name);
    }
}
