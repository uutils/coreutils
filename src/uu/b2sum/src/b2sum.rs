// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) algo

use clap::Command;
use uucore::checksum::cli::{checksum_main, options, standalone_checksum_app_with_length};
use uucore::checksum::compute::OutputFormat;
use uucore::checksum::{AlgoKind, calculate_blake2b_length_str};
use uucore::error::UResult;
use uucore::translate;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;
    let algo = Some(AlgoKind::Blake2b);

    let length = matches
        .get_one::<String>(options::LENGTH)
        .map(String::as_str)
        .map(calculate_blake2b_length_str)
        .transpose()?
        .flatten();

    let format = OutputFormat::from_standalone(std::env::args_os().into_iter());

    checksum_main(algo, length, matches, format?)
}

#[inline]
pub fn uu_app() -> Command {
    standalone_checksum_app_with_length(translate!("b2sum-about"), translate!("b2sum-usage"))
    .after_help(translate!("b2sum-after-help"))
}
