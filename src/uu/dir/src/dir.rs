// * This file is part of the uutils coreutils package.
// *
// * (c) gmnsii <gmnsii@protonmail.com>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

use clap::Command;
use std::path::Path;
use uu_ls::{options, Config, Format};
use uucore::error::UResult;
use uucore::quoting_style::{Quotes, QuotingStyle};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let command = uu_ls::uu_app();

    let matches = command.get_matches_from(args);

    let mut default_quoting_style = false;
    let mut default_format_style = false;

    // We check if any options on formatting or quoting style have been given.
    // If not, we will use dir default formatting and quoting style options

    if !matches.contains_id(options::QUOTING_STYLE)
        && !matches.contains_id(options::quoting::C)
        && !matches.contains_id(options::quoting::ESCAPE)
        && !matches.contains_id(options::quoting::LITERAL)
    {
        default_quoting_style = true;
    }
    if !matches.contains_id(options::FORMAT)
        && !matches.contains_id(options::format::ACROSS)
        && !matches.contains_id(options::format::COLUMNS)
        && !matches.contains_id(options::format::COMMAS)
        && !matches.contains_id(options::format::LONG)
        && !matches.contains_id(options::format::LONG_NO_GROUP)
        && !matches.contains_id(options::format::LONG_NO_OWNER)
        && !matches.contains_id(options::format::LONG_NUMERIC_UID_GID)
        && !matches.contains_id(options::format::ONE_LINE)
    {
        default_format_style = true;
    }

    let mut config = Config::from(&matches)?;

    if default_quoting_style {
        config.quoting_style = QuotingStyle::C {
            quotes: Quotes::None,
        };
    }
    if default_format_style {
        config.format = Format::Columns;
    }

    let locs = matches
        .values_of_os(options::PATHS)
        .map(|v| v.map(Path::new).collect())
        .unwrap_or_else(|| vec![Path::new(".")]);

    uu_ls::list(locs, &config)
}

// To avoid code duplication, we reuse ls uu_app function which has the same
// arguments. However, coreutils won't compile if one of the utils is missing
// an uu_app function, so we need this dummy one.
pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
}
