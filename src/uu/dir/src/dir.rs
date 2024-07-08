// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::OsString;
use std::path::Path;
use uu_ls::ls::{Config, Format};
use uu_ls::options;
use uucore::error::UResult;
use uucore::quoting_style::{Quotes, QuotingStyle};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let command = crate::uu_app();

    let matches = command.get_matches_from(args);

    let mut default_quoting_style = false;
    let mut default_format_style = false;

    // We check if any options on formatting or quoting style have been given.
    // If not, we will use dir default formatting and quoting style options

    if !matches.contains_id(options::QUOTING_STYLE)
        && !matches.get_flag(options::quoting::C)
        && !matches.get_flag(options::quoting::ESCAPE)
        && !matches.get_flag(options::quoting::LITERAL)
    {
        default_quoting_style = true;
    }
    if !matches.contains_id(options::FORMAT)
        && !matches.get_flag(options::format::ACROSS)
        && !matches.get_flag(options::format::COLUMNS)
        && !matches.get_flag(options::format::COMMAS)
        && !matches.get_flag(options::format::LONG)
        && !matches.get_flag(options::format::LONG_NO_GROUP)
        && !matches.get_flag(options::format::LONG_NO_OWNER)
        && !matches.get_flag(options::format::LONG_NUMERIC_UID_GID)
        && !matches.get_flag(options::format::ONE_LINE)
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
        .get_many::<OsString>(options::PATHS)
        .map(|v| v.map(Path::new).collect())
        .unwrap_or_else(|| vec![Path::new(".")]);

    uu_ls::ls::list(locs, &config)
}
