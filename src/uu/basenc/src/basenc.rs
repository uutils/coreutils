// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//spell-checker:ignore (args) lsbf msbf

use uucore::base_common;
use uucore::base_common::{Config, BASE_CMD_PARSE_ERROR};

use uucore::{
    encoding::Format,
    error::{UResult, UUsageError},
};

use std::io::{stdin, Read};
use uucore::error::UClapError;

fn parse_cmd_args(args: impl uucore::Args) -> UResult<(Config, Format)> {
    let matches = crate::uu_app()
        .try_get_matches_from(args.collect_lossy())
        .with_exit_code(1)?;
    let format = crate::ENCODINGS
        .iter()
        .find(|encoding| matches.get_flag(encoding.0))
        .ok_or_else(|| UUsageError::new(BASE_CMD_PARSE_ERROR, "missing encoding type"))?
        .1;
    let config = Config::from(&matches)?;
    Ok((config, format))
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let (config, format) = parse_cmd_args(args)?;
    // Create a reference to stdin so we can return a locked stdin from
    // parse_base_cmd_args
    let stdin_raw = stdin();
    let mut input: Box<dyn Read> = base_common::get_input(&config, &stdin_raw)?;

    base_common::handle_input(
        &mut input,
        format,
        config.wrap_cols,
        config.ignore_garbage,
        config.decode,
    )
}
