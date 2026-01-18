// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use uucore::error::UResult;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    // Delegate to hashsum with b2sum binary name
    // hashsum's uumain has #[uucore::main] so it returns i32
    let exit_code = uu_hashsum::uumain(args);

    // Convert exit code back to UResult
    if exit_code == 0 {
        Ok(())
    } else {
        Err(uucore::error::USimpleError::new(exit_code, "b2sum failed"))
    }
}
