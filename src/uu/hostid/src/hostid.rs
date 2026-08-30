// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) gethostid

use clap::Command;
use core::ffi::c_long;
#[cfg(not(target_env = "ohos"))]
use libc::gethostid;
use std::io::{Write, stdout};
use uucore::{error::UResult, format_usage};

use uucore::translate;

// OHOS SDK libc no longer exports gethostid; replicate the glibc semantics:
// read /etc/hostid when present, otherwise hash the hostname.
#[cfg(target_env = "ohos")]
fn gethostid() -> c_long {
    use std::fs::read;
    if let Ok(data) = read("/etc/hostid") {
        if data.len() >= 4 {
            let n: u32 = u32::from_ne_bytes([data[0], data[1], data[2], data[3]]);
            return n as c_long;
        }
    }
    let mut name = [0u8; 256];
    if unsafe { libc::gethostname(name.as_mut_ptr() as *mut _, name.len()) } == 0 {
        let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
        let mut h: u32 = 0x811c9dc5;
        for &b in &name[..end] {
            h ^= b as u32;
            h = h.wrapping_mul(0x01000193);
        }
        return h as c_long;
    }
    0
}

#[uucore::main(no_signals)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    uucore::clap_localization::handle_clap_result(uu_app(), args)?;
    /*
     * POSIX says gethostid returns a "32-bit identifier" but is silent
     * whether it's sign-extended.  Turn off any sign-extension.  This
     * is a no-op unless unsigned int is wider than 32 bits.
     */

    let mut result: c_long = unsafe { gethostid() };

    #[allow(overflowing_literals)]
    let mask = 0xffff_ffff;

    result &= mask;
    writeln!(stdout().lock(), "{result:0>8x}")?;
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new("hostid")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("hostid"))
        .about(translate!("hostid-about"))
        .override_usage(format_usage(&translate!("hostid-usage")))
        .infer_long_args(true)
}
