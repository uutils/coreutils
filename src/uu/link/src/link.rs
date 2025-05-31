// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use clap::builder::ValueParser;
use clap::{Arg, Command};
use std::ffi::OsString;
use std::fs::hard_link;
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult};
use uucore::{format_usage, help_about, help_usage};

use uucore::locale::{self, get_message};

pub mod options {
    pub static FILES: &str = "FILES";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let files: Vec<_> = matches
        .get_many::<OsString>(options::FILES)
        .unwrap_or_default()
        .collect();
    let old = Path::new(files[0]);
    let new = Path::new(files[1]);

    hard_link(old, new)
        .map_err_context(|| format!("cannot create link {} to {}", new.quote(), old.quote()))
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(get_message("link-about"))
        .override_usage(format_usage(&get_message("link-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILES)
                .hide(true)
                .required(true)
                .num_args(2)
                .value_hint(clap::ValueHint::AnyPath)
                .value_parser(ValueParser::os_string()),
        )
}
