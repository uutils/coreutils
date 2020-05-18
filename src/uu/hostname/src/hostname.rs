#![crate_name = "uu_hostname"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alan Andrade <alan.andradec@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate clap;
extern crate libc;
#[cfg(windows)]
extern crate winapi;

#[macro_use]
extern crate uucore;

use clap::{App, Arg, ArgMatches};
use std::collections::hash_set::HashSet;
use std::io;
use std::iter::repeat;
use std::net::ToSocketAddrs;
use std::str;

#[cfg(windows)]
use uucore::wide::*;
#[cfg(windows)]
use winapi::shared::minwindef::MAKEWORD;
#[cfg(windows)]
use winapi::um::sysinfoapi::{ComputerNamePhysicalDnsHostname, SetComputerNameExW};
#[cfg(windows)]
use winapi::um::winsock2::{GetHostNameW, WSACleanup, WSAStartup};

#[cfg(not(windows))]
use libc::gethostname;
#[cfg(not(windows))]
use libc::sethostname;

static ABOUT: &str = "Print or set the system's host name.";
static VERSION: &str = env!("CARGO_PKG_VERSION");

static OPT_DOMAIN: &str = "domain";
static OPT_IP_ADDRESS: &str = "ip-address";
static OPT_FQDN: &str = "fqdn";
static OPT_SHORT: &str = "short";
static OPT_HOST: &str = "host";

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

fn get_usage() -> String {
    format!("{0} [OPTION]... [HOSTNAME]", executable!())
}
fn execute(args: Vec<String>) -> i32 {
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
        .get_matches_from(&args);

    match matches.value_of(OPT_HOST) {
        None => display_hostname(matches),
        Some(host) => {
            if let Err(err) = xsethostname(host) {
                show_error!("{}", err);
                1
            } else {
                0
            }
        }
    }
}

fn display_hostname(matches: ArgMatches) -> i32 {
    let hostname = return_if_err!(1, xgethostname());

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

#[cfg(not(windows))]
fn xgethostname() -> io::Result<String> {
    use std::ffi::CStr;

    let namelen = 256;
    let mut name: Vec<u8> = repeat(0).take(namelen).collect();
    let err = unsafe {
        gethostname(
            name.as_mut_ptr() as *mut libc::c_char,
            namelen as libc::size_t,
        )
    };

    if err == 0 {
        let null_pos = name.iter().position(|byte| *byte == 0).unwrap_or(namelen);
        if null_pos == namelen {
            name.push(0);
        }

        Ok(CStr::from_bytes_with_nul(&name[..=null_pos])
            .unwrap()
            .to_string_lossy()
            .into_owned())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(windows)]
fn xgethostname() -> io::Result<String> {
    let namelen = 256;
    let mut name: Vec<u16> = repeat(0).take(namelen).collect();
    let err = unsafe { GetHostNameW(name.as_mut_ptr(), namelen as libc::c_int) };

    if err == 0 {
        Ok(String::from_wide_null(&name))
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(windows))]
fn xsethostname(name: &str) -> io::Result<()> {
    let vec_name: Vec<libc::c_char> = name.bytes().map(|c| c as libc::c_char).collect();

    let err = unsafe { sethostname(vec_name.as_ptr(), vec_name.len() as _) };

    if err != 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn xsethostname(name: &str) -> io::Result<()> {
    use std::ffi::OsStr;

    let wide_name = OsStr::new(name).to_wide_null();

    let err = unsafe { SetComputerNameExW(ComputerNamePhysicalDnsHostname, wide_name.as_ptr()) };

    if err == 0 {
        // NOTE: the above is correct, failure is when the function returns 0 apparently
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
