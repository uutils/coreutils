// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore hashset Addrs addrs

#[cfg(not(any(target_os = "freebsd", target_os = "openbsd")))]
use std::net::ToSocketAddrs;
use std::str;
use std::{collections::hash_set::HashSet, ffi::OsString};

use clap::builder::ValueParser;
use clap::{Arg, ArgAction, ArgMatches, Command};

#[cfg(any(target_os = "freebsd", target_os = "openbsd"))]
use dns_lookup::lookup_host;
use uucore::translate;

use uucore::{
    error::{FromIo, UResult},
    format_usage,
};

static OPT_DOMAIN: &str = "domain";
static OPT_IP_ADDRESS: &str = "ip-address";
static OPT_FQDN: &str = "fqdn";
static OPT_SHORT: &str = "short";
static OPT_HOST: &str = "host";

#[cfg(windows)]
mod wsa {
    use std::io;

    use windows_sys::Win32::Networking::WinSock::{WSACleanup, WSADATA, WSAStartup};

    pub(super) struct WsaHandle(());

    pub(super) fn start() -> io::Result<WsaHandle> {
        let err = unsafe {
            let mut data = std::mem::MaybeUninit::<WSADATA>::uninit();
            WSAStartup(0x0202, data.as_mut_ptr())
        };
        if err == 0 {
            Ok(WsaHandle(()))
        } else {
            Err(io::Error::from_raw_os_error(err))
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
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    #[cfg(windows)]
    let _handle = wsa::start().map_err_context(|| translate!("hostname-error-winsock"))?;

    match matches.get_one::<OsString>(OPT_HOST) {
        None => display_hostname(&matches),
        Some(host) => {
            hostname::set(host).map_err_context(|| translate!("hostname-error-set-hostname"))
        }
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("hostname-about"))
        .override_usage(format_usage(&translate!("hostname-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_DOMAIN)
                .short('d')
                .long("domain")
                .overrides_with_all([OPT_DOMAIN, OPT_IP_ADDRESS, OPT_FQDN, OPT_SHORT])
                .help(translate!("hostname-help-domain"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_IP_ADDRESS)
                .short('i')
                .long("ip-address")
                .overrides_with_all([OPT_DOMAIN, OPT_IP_ADDRESS, OPT_FQDN, OPT_SHORT])
                .help(translate!("hostname-help-ip-address"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_FQDN)
                .short('f')
                .long("fqdn")
                .overrides_with_all([OPT_DOMAIN, OPT_IP_ADDRESS, OPT_FQDN, OPT_SHORT])
                .help(translate!("hostname-help-fqdn"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_SHORT)
                .short('s')
                .long("short")
                .overrides_with_all([OPT_DOMAIN, OPT_IP_ADDRESS, OPT_FQDN, OPT_SHORT])
                .help(translate!("hostname-help-short"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_HOST)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::Hostname),
        )
}

fn display_hostname(matches: &ArgMatches) -> UResult<()> {
    let hostname = hostname::get()
        .map_err_context(|| "failed to get hostname".to_owned())?
        .to_string_lossy()
        .into_owned();

    if matches.get_flag(OPT_IP_ADDRESS) {
        let addresses;

        #[cfg(not(any(target_os = "freebsd", target_os = "openbsd")))]
        {
            let hostname = hostname + ":1";
            let addrs = hostname
                .to_socket_addrs()
                .map_err_context(|| "failed to resolve socket addresses".to_owned())?;
            addresses = addrs;
        }

        // DNS reverse lookup via "hostname:1" does not work on FreeBSD and OpenBSD
        // use dns-lookup crate instead
        #[cfg(any(target_os = "freebsd", target_os = "openbsd"))]
        {
            let addrs: Vec<std::net::IpAddr> = lookup_host(hostname.as_str())
                .map_err_context(|| "failed to lookup hostname".to_owned())?
                .collect();
            addresses = addrs;
        }

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
        if matches.get_flag(OPT_SHORT) || matches.get_flag(OPT_DOMAIN) {
            let mut it = hostname.char_indices().filter(|&ci| ci.1 == '.');
            if let Some(ci) = it.next() {
                if matches.get_flag(OPT_SHORT) {
                    println!("{}", &hostname[0..ci.0]);
                } else {
                    println!("{}", &hostname[ci.0 + 1..]);
                }
            } else if matches.get_flag(OPT_SHORT) {
                println!("{hostname}");
            }
            return Ok(());
        }

        println!("{hostname}");

        Ok(())
    }
}
