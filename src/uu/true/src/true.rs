//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

use clap::{App, AppSettings};
use uucore::executable;

pub fn uumain(_: impl uucore::Args) -> i32 {
    0
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(executable!())
        .setting(AppSettings::DisableHelpFlags)
        .setting(AppSettings::DisableVersion)
}
