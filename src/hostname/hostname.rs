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

extern crate libc;

#[macro_use]
extern crate uucore;

use std::collections::hash_set::HashSet;
use std::iter::repeat;
use std::str;
use std::io::Write;
use std::net::ToSocketAddrs;

static SYNTAX: &'static str = "[OPTION]... [HOSTNAME]"; 
static SUMMARY: &'static str = "Print or set the system's host name."; 
static LONG_HELP: &'static str = ""; 

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
    let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
        .optflag("d", "domain", "Display the name of the DNS domain if possible")
        .optflag("i", "ip-address", "Display the network address(es) of the host")
        .optflag("f", "fqdn", "Display the FQDN (Fully Qualified Domain Name) (default)")   // TODO: support --long
        .optflag("s", "short", "Display the short hostname (the portion before the first dot) if possible")
        .parse(args);

    match matches.free.len() {
        0 => {
            let hostname = xgethostname();

            if matches.opt_present("i") {
                // XXX: to_socket_addrs needs hostname:port so append a dummy port and remove it later.
                // This should use std::net::lookup_host, but that is still marked unstable.
                let hostname = hostname + ":1";
                match hostname.to_socket_addrs() {
                    Ok(addresses) => {
                        let mut hashset = HashSet::new();
                        let mut output = String::new();
                        for addr in addresses {
                            // XXX: not sure why this is necessary...
                            if !hashset.contains(&addr) {
                                let mut ip = format!("{}", addr);
                                if ip.ends_with(":1") {
                                    ip = ip[..ip.len()-2].to_owned();
                                }
                                output.push_str(&ip);
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
        _ => crash!(1, "{}", msg_wrong_number_of_arguments!(0, 1))
    };

    0
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

    str::from_utf8(&name[..last_char]).unwrap().to_owned()
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
