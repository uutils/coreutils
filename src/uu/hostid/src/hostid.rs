// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) gethostid

use libc::c_long;
use uucore::error::UResult;

// currently rust libc interface doesn't include gethostid
extern "C" {
    pub fn gethostid() -> c_long;
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    crate::uu_app().try_get_matches_from(args)?;
    hostid();
    Ok(())
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
