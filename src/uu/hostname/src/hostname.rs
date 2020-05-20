#![crate_name = "uu_hostname"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alan Andrade <alan.andradec@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate hostname;
extern crate libc;
#[cfg(windows)]
extern crate winapi;

#[macro_use]
extern crate uucore;

use getopts::Matches;
use std::collections::hash_set::HashSet;
use std::ffi::OsStr;
use std::net::ToSocketAddrs;
use std::str;

#[cfg(windows)]
use winapi::shared::minwindef::MAKEWORD;
#[cfg(windows)]
use winapi::um::winsock2::{WSACleanup, WSAStartup};

const SYNTAX: &str = "[OPTION]... [HOSTNAME]";
const SUMMARY: &str = "Print or set the system's host name.";
const LONG_HELP: &str = "";

pub fn uumain(args: Vec<String>) -> i32 {
    #![allow(clippy::let_and_return)]
    #[cfg(windows)]
    unsafe {
        #[allow(deprecated)]
        let mut data = std::mem::uninitialized();
        if WSAStartup(MAKEWORD(2, 2), &mut data as *mut _) != 0 {
            eprintln!("Failed to start Winsock 2.2");
            return 1;
        }
    }
    let result = execute(args);
    #[cfg(windows)]
    unsafe {
        WSACleanup();
    }
    result
}

fn execute(args: Vec<String>) -> i32 {
    let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
        .optflag(
            "d",
            "domain",
            "Display the name of the DNS domain if possible",
        )
        .optflag(
            "i",
            "ip-address",
            "Display the network address(es) of the host",
        )
        // TODO: support --long
        .optflag(
            "f",
            "fqdn",
            "Display the FQDN (Fully Qualified Domain Name) (default)",
        )
        .optflag(
            "s",
            "short",
            "Display the short hostname (the portion before the first dot) if \
             possible",
        )
        .parse(args);

    match matches.free.len() {
        0 => display_hostname(matches),
        1 => {
            if let Err(err) = hostname::set(OsStr::new(matches.free.last().unwrap())) {
                show_error!("{}", err);
                1
            } else {
                0
            }
        }
        _ => {
            show_error!("{}", msg_wrong_number_of_arguments!(0, 1));
            1
        }
    }
}

fn display_hostname(matches: Matches) -> i32 {
    let hostname = hostname::get().unwrap().into_string().unwrap();

    if matches.opt_present("i") {
        // XXX: to_socket_addrs needs hostname:port so append a dummy port and remove it later.
        // This was originally supposed to use std::net::lookup_host, but that seems to be
        // deprecated.  Perhaps we should use the dns-lookup crate?
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
                            let len = ip.len();
                            ip.truncate(len - 2);
                        }
                        output.push_str(&ip);
                        output.push_str(" ");
                        hashset.insert(addr);
                    }
                }
                let len = output.len();
                if len > 0 {
                    println!("{}", &output[0..len - 1]);
                }

                0
            }
            Err(f) => {
                show_error!("{}", f);

                1
            }
        }
    } else {
        if matches.opt_present("s") || matches.opt_present("d") {
            let mut it = hostname.char_indices().filter(|&ci| ci.1 == '.');
            if let Some(ci) = it.next() {
                if matches.opt_present("s") {
                    println!("{}", &hostname[0..ci.0]);
                } else {
                    println!("{}", &hostname[ci.0 + 1..]);
                }
                return 0;
            }
        }

        println!("{}", hostname);

        0
    }
}
