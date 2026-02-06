// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) algo

use clap::Command;

use uu_checksum_common::{standalone_checksum_app_with_length, standalone_with_length_main};

use uucore::checksum::{AlgoKind, calculate_blake2b_length_str};
use uucore::error::UResult;
use uucore::translate;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    standalone_with_length_main(
        AlgoKind::Blake2b,
        uu_app(),
        args,
        calculate_blake2b_length_str,
    )
}

#[inline]
pub fn uu_app() -> Command {
    standalone_checksum_app_with_length(translate!("b2sum-about"), translate!("b2sum-usage"))
}
