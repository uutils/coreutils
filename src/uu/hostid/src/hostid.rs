// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) gethostid

use clap::Command;
use libc::{c_long, gethostid};
use uucore::{error::UResult, format_usage};

use uucore::locale::get_message;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    uu_app().try_get_matches_from(args)?;
    hostid();
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(get_message("hostid-about"))
        .override_usage(format_usage(&get_message("hostid-usage")))
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
