// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::builder::ValueParser;
use clap::{Arg, Command};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::hard_link;
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult};
use uucore::format_usage;
use uucore::locale::{get_message, get_message_with_args};

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

    hard_link(old, new).map_err_context(|| {
        get_message_with_args(
            "link-error-cannot-create-link",
            HashMap::from([
                ("new".to_string(), new.quote().to_string()),
                ("old".to_string(), old.quote().to_string()),
            ]),
        )
    })
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
