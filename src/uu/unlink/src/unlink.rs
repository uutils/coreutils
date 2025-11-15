// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::ffi::OsString;
use std::fs::remove_file;
use std::path::Path;

use clap::builder::ValueParser;
use clap::{Arg, Command};

use uucore::display::Quotable;
use uucore::error::{FromIo, UResult};
use uucore::format_usage;
use uucore::translate;

static OPT_PATH: &str = "FILE";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let path: &Path = matches.get_one::<OsString>(OPT_PATH).unwrap().as_ref();

    remove_file(path)
        .map_err_context(|| translate!("unlink-error-cannot-unlink", "path" => path.quote()))
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("unlink-about"))
        .override_usage(format_usage(&translate!("unlink-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_PATH)
                .required(true)
                .hide(true)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::AnyPath),
        )
}
