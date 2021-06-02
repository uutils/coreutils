// This file is part of the uutils coreutils package.
//
// (c) Smigle00 <smigle00@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use platform_info::*;

use clap::{crate_version, App};

static ABOUT: &str = "Display machine architecture";
static SUMMARY: &str = "Determine architecture name for current machine.";

pub fn uumain(args: impl uucore::Args) -> i32 {
    App::new(executable!())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(SUMMARY)
        .get_matches_from(args);

    let uts = return_if_err!(1, PlatformInfo::new());
    println!("{}", uts.machine().trim());
    0
}
