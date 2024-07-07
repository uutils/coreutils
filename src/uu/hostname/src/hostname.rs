// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) MAKEWORD addrs hashset

use clap::ArgMatches;
use std::net::ToSocketAddrs;
use std::{collections::hash_set::HashSet, ffi::OsString};

use uucore::error::{FromIo, UResult};

#[cfg(windows)]
mod wsa {
    use std::io;

    use windows_sys::Win32::Networking::WinSock::{WSACleanup, WSAStartup, WSADATA};

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
    let matches = crate::uu_app().try_get_matches_from(args)?;

    #[cfg(windows)]
    let _handle = wsa::start().map_err_context(|| "failed to start Winsock".to_owned())?;

    match matches.get_one::<OsString>(crate::options::OPT_HOST) {
        None => display_hostname(&matches),
        Some(host) => hostname::set(host).map_err_context(|| "failed to set hostname".to_owned()),
    }
}

fn display_hostname(matches: &ArgMatches) -> UResult<()> {
    let hostname = hostname::get()
        .map_err_context(|| "failed to get hostname".to_owned())?
        .to_string_lossy()
        .into_owned();

    if matches.get_flag(crate::options::OPT_IP_ADDRESS) {
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
        if matches.get_flag(crate::options::OPT_SHORT)
            || matches.get_flag(crate::options::OPT_DOMAIN)
        {
            let mut it = hostname.char_indices().filter(|&ci| ci.1 == '.');
            if let Some(ci) = it.next() {
                if matches.get_flag(crate::options::OPT_SHORT) {
                    println!("{}", &hostname[0..ci.0]);
                } else {
                    println!("{}", &hostname[ci.0 + 1..]);
                }
                return Ok(());
            }
        }

        println!("{hostname}");

        Ok(())
    }
}
