// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::Command;
use std::ffi::OsString;
use std::path::Path;
use uu_ls::{Config, options};
use uucore::{error::UResult, format_usage, translate};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let command = uu_app();

    // The arguments are kept for the caret in SIZE diagnostics, which echoes
    // the command line.
    let (matches, diag_args) =
        uucore::clap_localization::handle_clap_result_with_diagnostics(command, args.collect(), 2)?;

    let config = Config::from_dir(&matches, diag_args.as_deref())?;

    let locs = matches
        .get_many::<OsString>(options::PATHS)
        .map_or_else(|| vec![Path::new(".")], |v| v.map(Path::new).collect());

    uu_ls::list(locs, &config)
}

// To avoid code duplication, we reuse ls uu_app function which has the same
// arguments. However, coreutils won't compile if one of the utils is missing
// an uu_app function, so we return the `ls` app.
pub fn uu_app() -> Command {
    uu_ls::uu_app()
        .name("dir")
        .override_usage(format_usage(&translate!("dir-usage")))
        .about(translate!("dir-about"))
}
