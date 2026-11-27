// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::Command;
use uucore::error::UResult;
use uucore::{format_usage, translate};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    uu_ls::run(args, uu_ls::ProgramMode::Vdir, uu_app)
}

// To avoid code duplication, we reuse ls uu_app function which has the same
// arguments. However, coreutils won't compile if one of the utils is missing
// an uu_app function, so we need this dummy one.
pub fn uu_app() -> Command {
    uu_ls::uu_app()
        .name("vdir")
        .override_usage(format_usage(&translate!("vdir-usage")))
        .about(translate!("vdir-about"))
}
