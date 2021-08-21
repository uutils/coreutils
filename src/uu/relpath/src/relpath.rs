//  * This file is part of the uutils coreutils package.
//  *
//  * (c) 2014 Vsevolod Velichko <torkvemada@sorokdva.net>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) subpath absto absfrom absbase

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg};
use std::env;
use std::path::{Path, PathBuf};
use uucore::fs::{canonicalize, MissingHandling, ResolveMode};
use uucore::InvalidEncodingHandling;

static ABOUT: &str = "Convert TO destination to the relative path from the FROM dir.
If FROM path is omitted, current working dir will be used.";

mod options {
    pub const DIR: &str = "DIR";
    pub const TO: &str = "TO";
    pub const FROM: &str = "FROM";
}

fn get_usage() -> String {
    format!("{} [-d DIR] TO [FROM]", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();
    let usage = get_usage();

    let matches = uu_app().usage(&usage[..]).get_matches_from(args);

    let to = Path::new(matches.value_of(options::TO).unwrap()).to_path_buf(); // required
    let from = match matches.value_of(options::FROM) {
        Some(p) => Path::new(p).to_path_buf(),
        None => env::current_dir().unwrap(),
    };
    let absto = canonicalize(to, MissingHandling::Normal, ResolveMode::Logical).unwrap();
    let absfrom = canonicalize(from, MissingHandling::Normal, ResolveMode::Logical).unwrap();

    if matches.is_present(options::DIR) {
        let base = Path::new(&matches.value_of(options::DIR).unwrap()).to_path_buf();
        let absbase = canonicalize(base, MissingHandling::Normal, ResolveMode::Logical).unwrap();
        if !absto.as_path().starts_with(absbase.as_path())
            || !absfrom.as_path().starts_with(absbase.as_path())
        {
            println!("{}", absto.display());
            return 0;
        }
    }

    let mut suffix_pos = 0;
    for (f, t) in absfrom.components().zip(absto.components()) {
        if f == t {
            suffix_pos += 1;
        } else {
            break;
        }
    }

    let mut result = PathBuf::new();
    absfrom
        .components()
        .skip(suffix_pos)
        .map(|_| result.push(".."))
        .last();
    absto
        .components()
        .skip(suffix_pos)
        .map(|x| result.push(x.as_os_str()))
        .last();

    println!("{}", result.display());
    0
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(executable!())
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::DIR)
                .short("d")
                .takes_value(true)
                .help("If any of FROM and TO is not subpath of DIR, output absolute path instead of relative"),
        )
        .arg(
            Arg::with_name(options::TO)
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::FROM)
                .takes_value(true),
        )
}
