// * This file is part of the uutils coreutils package.
// *
// * (c) gmnsii <gmnsii@protonmail.com>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

use clap::Command;
use std::path::Path;
use uu_ls::quoting_style::{Quotes, QuotingStyle};
use uu_ls::{options, Config, Format};
use uucore::error::UResult;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let command = uu_ls::uu_app();

    let matches = command.get_matches_from(args);

    let mut default_quoting_style = false;
    let mut default_format_style = false;

    // We check if any options on formatting or quoting style have been given.
    // If not, we will use dir default formatting and quoting style options

    if !matches.is_present(options::QUOTING_STYLE)
        && !matches.is_present(options::quoting::C)
        && !matches.is_present(options::quoting::ESCAPE)
        && !matches.is_present(options::quoting::LITERAL)
    {
        default_quoting_style = true;
    }
    if !matches.is_present(options::FORMAT)
        && !matches.is_present(options::format::ACROSS)
        && !matches.is_present(options::format::COLUMNS)
        && !matches.is_present(options::format::COMMAS)
        && !matches.is_present(options::format::LONG)
        && !matches.is_present(options::format::LONG_NO_GROUP)
        && !matches.is_present(options::format::LONG_NO_OWNER)
        && !matches.is_present(options::format::LONG_NUMERIC_UID_GID)
        && !matches.is_present(options::format::ONE_LINE)
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
