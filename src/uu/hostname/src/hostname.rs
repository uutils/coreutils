//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alan Andrade <alan.andradec@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) MAKEWORD addrs hashset

use std::collections::hash_set::HashSet;
use std::net::ToSocketAddrs;
use std::str;

use clap::{crate_version, Arg, ArgMatches, Command};

use uucore::{
    error::{FromIo, UResult},
    format_usage,
};

static ABOUT: &str = "Display or set the system's host name.";
const USAGE: &str = "{} [OPTION]... [HOSTNAME]";

static OPT_DOMAIN: &str = "domain";
static OPT_IP_ADDRESS: &str = "ip-address";
static OPT_FQDN: &str = "fqdn";
static OPT_SHORT: &str = "short";
static OPT_HOST: &str = "host";

#[cfg(windows)]
mod wsa {
    use std::io;

    use winapi::shared::minwindef::MAKEWORD;
    use winapi::um::winsock2::{WSACleanup, WSAStartup, WSADATA};

    pub(super) struct WsaHandle(());

    pub(super) fn start() -> io::Result<WsaHandle> {
        let err = unsafe {
            let mut data = std::mem::MaybeUninit::<WSADATA>::uninit();
            WSAStartup(MAKEWORD(2, 2), data.as_mut_ptr())
        };
        if err != 0 {
            Err(io::Error::from_raw_os_error(err))
        } else {
            Ok(WsaHandle(()))
        }
    }

    impl Drop for WsaHandle {
        fn drop(&mut self) {
            unsafe {
                // This possibly returns an error but we can't handle it
                let _err = WSACleanup();
            }
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    #[cfg(windows)]
    let _handle = wsa::start().map_err_context(|| "failed to start Winsock".to_owned())?;

    match matches.value_of_os(OPT_HOST) {
        None => display_hostname(&matches),
        Some(host) => hostname::set(host).map_err_context(|| "failed to set hostname".to_owned()),
    }
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_DOMAIN)
                .short('d')
                .long("domain")
                .overrides_with_all(&[OPT_DOMAIN, OPT_IP_ADDRESS, OPT_FQDN, OPT_SHORT])
                .help("Display the name of the DNS domain if possible"),
        )
        .arg(
            Arg::new(OPT_IP_ADDRESS)
                .short('i')
                .long("ip-address")
                .overrides_with_all(&[OPT_DOMAIN, OPT_IP_ADDRESS, OPT_FQDN, OPT_SHORT])
                .help("Display the network address(es) of the host"),
        )
        .arg(
            Arg::new(OPT_FQDN)
                .short('f')
                .long("fqdn")
                .overrides_with_all(&[OPT_DOMAIN, OPT_IP_ADDRESS, OPT_FQDN, OPT_SHORT])
                .help("Display the FQDN (Fully Qualified Domain Name) (default)"),
        )
        .arg(
            Arg::new(OPT_SHORT)
                .short('s')
                .long("short")
                .overrides_with_all(&[OPT_DOMAIN, OPT_IP_ADDRESS, OPT_FQDN, OPT_SHORT])
                .help("Display the short hostname (the portion before the first dot) if possible"),
        )
        .arg(Arg::new(OPT_HOST).allow_invalid_utf8(true))
}

fn display_hostname(matches: &ArgMatches) -> UResult<()> {
    let hostname = hostname::get()
        .map_err_context(|| "failed to get hostname".to_owned())?
        .to_string_lossy()
        .into_owned();

    if matches.is_present(OPT_IP_ADDRESS) {
        // XXX: to_socket_addrs needs hostname:port so append a dummy port and remove it later.
        // This was originally supposed to use std::net::lookup_host, but that seems to be
        // deprecated.  Perhaps we should use the dns-lookup crate?
        let hostname = hostname + ":1";
        let addresses = hostname
            .to_socket_addrs()
            .map_err_context(|| "failed to resolve socket addresses".to_owned())?;
        let mut hashset = HashSet::new();
        let mut output = String::new();
        for addr in addresses {
            // XXX: not sure why this is necessary...
            if !hashset.contains(&addr) {
                let mut ip = addr.to_string();
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
