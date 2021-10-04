//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

/* last synced with: whoami (GNU coreutils) 8.21 */

#[macro_use]
extern crate clap;

use clap::App;

use uucore::display::println_verbatim;
use uucore::error::{FromIo, UResult};

mod platform;

static ABOUT: &str = "Print the current username.";

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    uu_app().get_matches_from(args);
    let username = platform::get_username().map_err_context(|| "failed to get username".into())?;
    println_verbatim(&username).map_err_context(|| "failed to print username".into())?;
    Ok(())
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
}
