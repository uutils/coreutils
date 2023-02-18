//  * This file is part of the uutils coreutils package.
//  *
//  * (c) 2014 Vsevolod Velichko <torkvemada@sorokdva.net>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) subpath absto absfrom absbase

use clap::{crate_version, Arg, Command};
use std::env;
use std::path::{Path, PathBuf};
use uucore::display::println_verbatim;
use uucore::error::{FromIo, UResult};
use uucore::format_usage;
use uucore::fs::{canonicalize, MissingHandling, ResolveMode};

static ABOUT: &str = "Convert TO destination to the relative path from the FROM dir.
If FROM path is omitted, current working dir will be used.";
const USAGE: &str = "{} [-d DIR] TO [FROM]";

mod options {
    pub const DIR: &str = "DIR";
    pub const TO: &str = "TO";
    pub const FROM: &str = "FROM";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_lossy();

    let matches = uu_app().get_matches_from(args);

    let to = Path::new(matches.get_one::<String>(options::TO).unwrap()).to_path_buf(); // required
    let from = match matches.get_one::<String>(options::FROM) {
        Some(p) => Path::new(p).to_path_buf(),
        None => env::current_dir().unwrap(),
    };
    let absto = canonicalize(to, MissingHandling::Normal, ResolveMode::Logical)
        .map_err_context(String::new)?;
    let absfrom = canonicalize(from, MissingHandling::Normal, ResolveMode::Logical)
        .map_err_context(String::new)?;

    if matches.contains_id(options::DIR) {
        let base = Path::new(&matches.get_one::<String>(options::DIR).unwrap()).to_path_buf();
        let absbase = canonicalize(base, MissingHandling::Normal, ResolveMode::Logical)
            .map_err_context(String::new)?;
        if !absto.as_path().starts_with(absbase.as_path())
            || !absfrom.as_path().starts_with(absbase.as_path())
        {
            return println_verbatim(absto).map_err_context(String::new);
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

    println_verbatim(result).map_err_context(String::new)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(Arg::new(options::DIR).short('d').help(
            "If any of FROM and TO is not subpath of DIR, output absolute path instead of relative",
        ))
        .arg(Arg::new(options::TO).value_hint(clap::ValueHint::AnyPath))
        .arg(Arg::new(options::FROM).value_hint(clap::ValueHint::AnyPath))
}
