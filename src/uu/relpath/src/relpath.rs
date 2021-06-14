//  * This file is part of the uutils coreutils package.
//  *
//  * (c) 2014 Vsevolod Velichko <torkvemada@sorokdva.net>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) subpath absto absfrom absbase

#[macro_use]
extern crate uucore;

use std::env;
use std::path::{Path, PathBuf};
use uucore::fs::{canonicalize, CanonicalizeMode};
use uucore::InvalidEncodingHandling;

use crate::app::{get_app, options};

mod app;

fn get_usage() -> String {
    format!("{} [-d DIR] TO [FROM]", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();
    let usage = get_usage();

    let matches = get_app(executable!())
        .usage(&usage[..])
        .get_matches_from(args);

    let to = Path::new(matches.value_of(options::TO).unwrap()).to_path_buf(); // required
    let from = match matches.value_of(options::FROM) {
        Some(p) => Path::new(p).to_path_buf(),
        None => env::current_dir().unwrap(),
    };
    let absto = canonicalize(to, CanonicalizeMode::Normal).unwrap();
    let absfrom = canonicalize(from, CanonicalizeMode::Normal).unwrap();

    if matches.is_present(options::DIR) {
        let base = Path::new(&matches.value_of(options::DIR).unwrap()).to_path_buf();
        let absbase = canonicalize(base, CanonicalizeMode::Normal).unwrap();
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
