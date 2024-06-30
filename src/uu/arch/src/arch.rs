// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use platform_info::*;

use uucore::error::{UResult, USimpleError};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    crate::uu_app().try_get_matches_from(args)?;

    let uts = PlatformInfo::new().map_err(|_e| USimpleError::new(1, "cannot get system name"))?;

    println!("{}", uts.machine().to_string_lossy().trim());
    Ok(())
}
