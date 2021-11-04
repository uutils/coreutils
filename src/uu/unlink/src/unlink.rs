//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Colin Warren <me@zv.ms>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

/* last synced with: unlink (GNU coreutils) 8.21 */

use std::fs::remove_file;
use std::path::Path;

use clap::{crate_version, App, Arg};

use uucore::display::Quotable;
use uucore::error::{FromIo, UResult};

static ABOUT: &str = "Unlink the file at FILE.";
static OPT_PATH: &str = "FILE";

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let path: &Path = matches.value_of_os(OPT_PATH).unwrap().as_ref();

    remove_file(path).map_err_context(|| format!("cannot unlink {}", path.quote()))
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .arg(Arg::with_name(OPT_PATH).required(true).hidden(true))
}
