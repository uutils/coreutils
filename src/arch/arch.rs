#![crate_name = "uu_arch"]

// This file is part of the uutils coreutils package.
//
// (c) Smigle00 <smigle00@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//

#[macro_use]
extern crate uucore;
use uucore::utsname::Uname;

static SYNTAX: &'static str = "";
static SUMMARY: &'static str = "Determine architecture name for current machine.";
static LONG_HELP: &'static str = "";

pub fn uumain(args: Vec<String>) -> i32 {
    new_coreopts!(SYNTAX, SUMMARY, LONG_HELP).parse(args);
    let uts = Uname::new();
    println!("{}", uts.machine().trim());
    0
}
