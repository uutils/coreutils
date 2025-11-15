// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// Specific implementation for OpenBSD: tool unsupported (utmpx not supported)

use crate::uu_app;

use uucore::error::UResult;
use uucore::translate;

pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let _matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;
    println!("{}", translate!("who-unsupported-openbsd"));
    Ok(())
}
