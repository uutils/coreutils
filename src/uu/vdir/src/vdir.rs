// * This file is part of the uutils coreutils package.
// *
// * (c) gmnsii <gmnsii@protonmail.com>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

use clap::Command;
use uucore::error::UResult;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    uu_ls::vdir_main(args)
}

// Coreutils won't compile if not every util have an uu_app function.
// However, we can't put it here since it needs to be in the same place as
// the entry point for the vdir util, which is in ls/ls.rs. We could put the
// entry point here, but we would need a lot of refactoring.
// To make our life easier, we use this dummy function.
pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
}
