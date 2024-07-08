// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::env;
use uucore::error::UResult;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = crate::uu_app().get_matches_from(args);

    let variables: Vec<String> = matches
        .get_many::<String>(crate::options::ARG_VARIABLES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let separator = if matches.get_flag(crate::options::OPT_NULL) {
        "\x00"
    } else {
        "\n"
    };

    if variables.is_empty() {
        for (env_var, value) in env::vars() {
            print!("{env_var}={value}{separator}");
        }
        return Ok(());
    }

    let mut error_found = false;
    for env_var in variables {
        // we silently ignore a=b as variable but we trigger an error
        if env_var.contains('=') {
            error_found = true;
            continue;
        }
        if let Ok(var) = env::var(env_var) {
            print!("{var}{separator}");
        } else {
            error_found = true;
        }
    }

    if error_found {
        Err(1.into())
    } else {
        Ok(())
    }
}
