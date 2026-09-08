// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// Specific implementation for OpenBSD: tool unsupported (utmpx not supported)

use crate::Who;
use uucore::error::UResult;
use uucore::translate;

impl Who {
    #[allow(
        clippy::unnecessary_wraps,
        reason = "signature shared across platforms"
    )]
    pub(crate) fn exec(&mut self) -> UResult<()> {
        println!("{}", translate!("who-unsupported-openbsd"));
        Ok(())
    }
}
