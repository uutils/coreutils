//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Michael Gehring <mg@ebfe.org>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
use clap::{crate_version, App, Arg};
use std::fs::hard_link;
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult};

static ABOUT: &str = "Call the link function to create a link named FILE2 to an existing FILE1.";

pub mod options {
    pub static FILES: &str = "FILES";
}

fn usage() -> String {
    format!("{0} FILE1 FILE2", uucore::execution_phrase())
}

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let usage = usage();
    let matches = uu_app().usage(&usage[..]).get_matches_from(args);

    let files: Vec<_> = matches
        .values_of_os(options::FILES)
        .unwrap_or_default()
        .collect();
    let old = Path::new(files[0]);
    let new = Path::new(files[1]);

    hard_link(old, new)
        .map_err_context(|| format!("cannot create link {} to {}", new.quote(), old.quote()))
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::FILES)
                .hidden(true)
                .required(true)
                .min_values(2)
                .max_values(2)
                .takes_value(true),
        )
}
