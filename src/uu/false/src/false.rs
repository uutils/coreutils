//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
#[macro_use]
extern crate uucore;

use clap::App;
use uucore::error::{UError, UResult};
use uucore::executable;

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    uu_app().get_matches_from(args);
    Err(UError::from(1))
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(executable!())
}
