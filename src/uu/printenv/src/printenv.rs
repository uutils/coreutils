//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

/* last synced with: printenv (GNU coreutils) 8.13 */

use clap::{crate_version, Arg, Command};
use std::env;
use uucore::{error::UResult, format_usage};

static ABOUT: &str = "Display the values of the specified environment VARIABLE(s), or (with no VARIABLE) display name and value pairs for them all.";
const USAGE: &str = "{} [VARIABLE]... [OPTION]...";

static OPT_NULL: &str = "null";

static ARG_VARIABLES: &str = "variables";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

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
        return Ok(());
    }

    let mut not_found = false;
    for env_var in variables {
        if let Ok(var) = env::var(env_var) {
            print!("{}{}", var, separator);
        } else {
            not_found = true;
        }
    }

    if not_found {
        Err(1.into())
    } else {
        Ok(())
    }
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_NULL)
                .short('0')
                .long(OPT_NULL)
                .help("end each output line with 0 byte rather than newline"),
        )
        .arg(
            Arg::new(ARG_VARIABLES)
                .multiple_occurrences(true)
                .takes_value(true)
                .min_values(1),
        )
}
