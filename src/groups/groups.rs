#![crate_name = "uu_groups"]

// This file is part of the uutils coreutils package.
//
// (c) Alan Andrade <alan.andradec@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
//

#[macro_use]
extern crate uucore;
use uucore::entries::{Passwd, Locate, get_groups, gid2grp};
use std::io::Write;

static SYNTAX: &'static str = "[user]";
static SUMMARY: &'static str = "display current group names";

pub fn uumain(args: Vec<String>) -> i32 {
    let matches = new_coreopts!(SYNTAX, SUMMARY, "")
          .parse(args);

    if matches.free.is_empty() {
        println!("{}", get_groups().unwrap().iter().map(|&g| gid2grp(g).unwrap()).collect::<Vec<_>>().join(" "));
    } else {
        if let Ok(p) = Passwd::locate(matches.free[0].as_str()) {
            println!("{}", p.belongs_to().iter().map(|&g| gid2grp(g).unwrap()).collect::<Vec<_>>().join(" "));
        } else {
            crash!(1, "unknown user {}", matches.free[0]);
        }
    }

    0
}
