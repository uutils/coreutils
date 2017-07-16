#![crate_name = "uu_users"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) KokaKiwi <kokakiwi@kokakiwi.net>
 * (c) Jian Zeng <anonymousknight86@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: whoami (GNU coreutils) 8.22 */

// Allow dead code here in order to keep all fields, constants here, for consistency.
#![allow(dead_code)]

extern crate getopts;
extern crate uucore;

use uucore::utmpx::*;

use getopts::Options;

static NAME: &'static str = "users";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!("{}", f),
    };

    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {} [OPTION]... [FILE]", NAME);
        println!("");
        println!("{}", opts.usage("Output who is currently logged in according to FILE."));
        return 0;
    }

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    let filename = if !matches.free.is_empty() {
        matches.free[0].as_ref()
    } else {
        DEFAULT_FILE
    };

    exec(filename);

    0
}

fn exec(filename: &str) {
    let mut users = Utmpx::iter_all_records()
        .read_from(filename)
        .filter(|ut| ut.is_user_process())
        .map(|ut| ut.user())
        .collect::<Vec<_>>();

    if !users.is_empty() {
        users.sort();
        println!("{}", users.join(" "));
    }
}
