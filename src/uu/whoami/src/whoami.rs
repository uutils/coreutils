//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

/* last synced with: whoami (GNU coreutils) 8.21 */

use clap::{crate_version, Command};

use uucore::display::println_verbatim;
use uucore::error::{FromIo, UResult};
use uucore::{format_usage, help_about, help_usage};

mod platform;

const ABOUT: &str = help_about!("whoami.md");
const USAGE: &str = help_usage!("whoami.md");

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    uu_app().try_get_matches_from(args)?;
    let username = platform::get_username().map_err_context(|| "failed to get username".into())?;
    println_verbatim(username).map_err_context(|| "failed to print username".into())?;
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
}
