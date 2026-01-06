// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) algo

use clap::Command;

use uu_checksum_common::{standalone_checksum_app, standalone_main};

use uucore::checksum::AlgoKind;
use uucore::error::UResult;
use uucore::translate;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    standalone_main(AlgoKind::Md5, uu_app(), args)
}

#[inline]
pub fn uu_app() -> Command {
    standalone_checksum_app(translate!("md5sum-about"), translate!("md5sum-usage"))
}
