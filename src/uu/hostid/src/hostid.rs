// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) gethostid

use clap::{crate_version, Command};
use libc::c_long;
use uucore::{error::UResult, format_usage, help_about, help_usage};

const USAGE: &str = help_usage!("hostid.md");
const ABOUT: &str = help_about!("hostid.md");

// currently rust libc interface doesn't include gethostid
extern "C" {
    pub fn gethostid() -> c_long;
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    uu_app().get_matches_from(args);
    hostid();
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
}

fn hostid() {
    /*
     * POSIX says gethostid returns a "32-bit identifier" but is silent
     * whether it's sign-extended.  Turn off any sign-extension.  This
     * is a no-op unless unsigned int is wider than 32 bits.
     */

    let mut result: c_long;
    unsafe {
        result = gethostid();
    }

    #[allow(overflowing_literals)]
    let mask = 0xffff_ffff;

    result &= mask;
    println!("{result:0>8x}");
}
