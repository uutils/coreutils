//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

/* last synced with: printenv (GNU coreutils) 8.13 */

#[macro_use]
extern crate uucore;

use std::env;

use crate::app::{get_app, ARG_VARIABLES, OPT_NULL};

pub mod app;

fn get_usage() -> String {
    format!("{0} [VARIABLE]... [OPTION]...", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let matches = get_app(executable!())
        .usage(&usage[..])
        .get_matches_from(args);

    let variables: Vec<String> = matches
        .values_of(ARG_VARIABLES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let separator = if matches.is_present(OPT_NULL) {
        "\x00"
    } else {
        "\n"
    };

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
