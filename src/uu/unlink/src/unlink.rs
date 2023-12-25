// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::OsString;
use std::fs::remove_file;
use std::path::Path;

use clap::builder::ValueParser;
use clap::{crate_version, Arg, Command};

use uucore::display::Quotable;
use uucore::error::{FromIo, UResult};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("unlink.md");
const USAGE: &str = help_usage!("unlink.md");
static OPT_PATH: &str = "FILE";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let path: &Path = matches.get_one::<OsString>(OPT_PATH).unwrap().as_ref();

    remove_file(path).map_err_context(|| format!("cannot unlink {}", path.quote()))
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_PATH)
                .required(true)
                .hide(true)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::AnyPath),
        )
}
