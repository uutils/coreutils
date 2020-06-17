// This file is part of the uutils coreutils package.
//
// (c) Alan Andrade <alan.andradec@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) passwd

#[macro_use]
extern crate uucore;
use uucore::entries::{get_groups, gid2grp, Locate, Passwd};

extern crate clap;
use clap::{App, Arg};

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "display current group names";
static OPT_USER: &str = "user";

fn get_usage() -> String {
    format!("{0} [USERNAME]", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(Arg::with_name(OPT_USER))
        .get_matches_from(args);

    match matches.value_of(OPT_USER) {
        None => {
            println!(
                "{}",
                get_groups()
                    .unwrap()
                    .iter()
                    .map(|&g| gid2grp(g).unwrap())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            0
        }
        Some(user) => {
            if let Ok(p) = Passwd::locate(user) {
                println!(
                    "{}",
                    p.belongs_to()
                        .iter()
                        .map(|&g| gid2grp(g).unwrap())
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                0
            } else {
                crash!(1, "unknown user {}", user);
            }
        }
    }
}
