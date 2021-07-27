//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alan Andrade <alan.andradec@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) MAKEWORD addrs hashset

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg, ArgMatches};
use std::collections::hash_set::HashSet;
use std::net::ToSocketAddrs;
use std::str;
#[cfg(windows)]
use uucore::error::UUsageError;
use uucore::error::{UResult, USimpleError};

#[cfg(windows)]
use winapi::shared::minwindef::MAKEWORD;
#[cfg(windows)]
use winapi::um::winsock2::{WSACleanup, WSAStartup};

static ABOUT: &str = "Display or set the system's host name.";

static OPT_DOMAIN: &str = "domain";
static OPT_IP_ADDRESS: &str = "ip-address";
static OPT_FQDN: &str = "fqdn";
static OPT_SHORT: &str = "short";
static OPT_HOST: &str = "host";

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    #![allow(clippy::let_and_return)]
    #[cfg(windows)]
    unsafe {
        #[allow(deprecated)]
        let mut data = std::mem::uninitialized();
        if WSAStartup(MAKEWORD(2, 2), &mut data as *mut _) != 0 {
            return Err(UUsageError::new(
                1,
                "Failed to start Winsock 2.2".to_string(),
            ));
        }
    }
    let result = execute(args);
    #[cfg(windows)]
    unsafe {
        WSACleanup();
    }
    result
}

fn usage() -> String {
    format!("{0} [OPTION]... [HOSTNAME]", execution_phrase!())
}

fn execute(args: impl uucore::Args) -> UResult<()> {
    let usage = usage();
    let matches = uu_app().usage(&usage[..]).get_matches_from(args);

    match matches.value_of(OPT_HOST) {
        None => display_hostname(&matches),
        Some(host) => {
            if let Err(err) = hostname::set(host) {
                return Err(USimpleError::new(1, format!("{}", err)));
            } else {
                Ok(())
            }
        }
    }
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(util_name!())
        .version(crate_version!())
        .about(ABOUT)
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
}

fn display_hostname(matches: &ArgMatches) -> UResult<()> {
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

                Ok(())
            }
            Err(f) => {
                return Err(USimpleError::new(1, format!("{}", f)));
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
                return Ok(());
            }
        }

        println!("{}", hostname);

        Ok(())
    }
}
