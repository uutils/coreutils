//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alan Andrade <alan.andradec@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) MAKEWORD addrs hashset

extern crate clap;
extern crate hostname;
extern crate libc;
#[cfg(windows)]
extern crate winapi;

#[macro_use]
extern crate uucore;

use clap::{App, Arg, ArgMatches};
use std::collections::hash_set::HashSet;
use std::net::ToSocketAddrs;
use std::str;

#[cfg(windows)]
use winapi::shared::minwindef::MAKEWORD;
#[cfg(windows)]
use winapi::um::winsock2::{WSACleanup, WSAStartup};

static ABOUT: &str = "Display or set the system's host name.";
static VERSION: &str = env!("CARGO_PKG_VERSION");

static OPT_DOMAIN: &str = "domain";
static OPT_IP_ADDRESS: &str = "ip-address";
static OPT_FQDN: &str = "fqdn";
static OPT_SHORT: &str = "short";
static OPT_HOST: &str = "host";

pub fn uumain(args: impl uucore::Args) -> i32 {
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

fn get_usage() -> String {
    format!("{0} [OPTION]... [HOSTNAME]", executable!())
}
fn execute(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(OPT_DOMAIN)
                .short("d")
                .long("domain")
                .help("Display the name of the DNS domain if possible"),
        )
        .arg(
            Arg::with_name(OPT_IP_ADDRESS)
                .short("i")
                .long("ip-address")
                .help("Display the network address(es) of the host"),
        )
        // TODO: support --long
        .arg(
            Arg::with_name(OPT_FQDN)
                .short("f")
                .long("fqdn")
                .help("Display the FQDN (Fully Qualified Domain Name) (default)"),
        )
        .arg(Arg::with_name(OPT_SHORT).short("s").long("short").help(
            "Display the short hostname (the portion before the first dot) if \
                possible",
        ))
        .arg(Arg::with_name(OPT_HOST))
        .get_matches_from(args);

    match matches.value_of(OPT_HOST) {
        None => display_hostname(&matches),
        Some(host) => {
            if let Err(err) = hostname::set(host) {
                show_error!("{}", err);
                1
            } else {
                0
            }
        }
    }
}

fn display_hostname(matches: &ArgMatches) -> i32 {
    let hostname = hostname::get().unwrap().into_string().unwrap();

    if matches.is_present(OPT_IP_ADDRESS) {
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
                        output.push(' ');
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
        if matches.is_present(OPT_SHORT) || matches.is_present(OPT_DOMAIN) {
            let mut it = hostname.char_indices().filter(|&ci| ci.1 == '.');
            if let Some(ci) = it.next() {
                if matches.is_present(OPT_SHORT) {
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
