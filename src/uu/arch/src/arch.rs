// This file is part of the uutils coreutils package.
//
// (c) Smigle00 <smigle00@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

extern crate platform_info;
#[macro_use]
extern crate uucore;

use platform_info::*;

static SYNTAX: &str = "Display machine architecture";
static SUMMARY: &str = "Determine architecture name for current machine.";
static LONG_HELP: &str = "";

pub fn uumain(args: impl uucore::Args) -> i32 {
    app!(SYNTAX, SUMMARY, LONG_HELP).parse(args.collect_str());
    let uts = return_if_err!(1, PlatformInfo::new());
    println!("{}", uts.machine().trim());
    0
}
