//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

/* last synced with: printenv (GNU coreutils) 8.13 */

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::env;

static ABOUT: &str = "Display the values of the specified environment VARIABLE(s), or (with no VARIABLE) display name and value pairs for them all.";
static VERSION: &str = env!("CARGO_PKG_VERSION");

static OPT_NULL: &str = "null";

static ARG_VARIABLES: &str = "variables";

fn get_usage() -> String {
    format!("{0} [VARIABLE]... [OPTION]...", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .arg(
            Arg::with_name(OPT_NULL)
                .short("0")
                .long(OPT_NULL)
                .help("end each output line with 0 byte rather than newline"),
        )
        .arg(
            Arg::with_name(ARG_VARIABLES)
                .multiple(true)
                .takes_value(true)
                .min_values(1),
        )
        .get_matches_from(args);

    let variables: Vec<String> = matches
        .values_of(ARG_VARIABLES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let mut separator = "\n";
    if matches.is_present(OPT_NULL) {
        separator = "\x00";
    }

    if variables.is_empty() {
        for (env_var, value) in env::vars() {
            print!("{}={}{}", env_var, value, separator);
        }
        return 0;
    }

    for env_var in variables {
        if let Ok(var) = env::var(env_var) {
            print!("{}{}", var, separator);
        }
    }
    0
}
