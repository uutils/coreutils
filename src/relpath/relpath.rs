#![crate_name = "uu_relpath"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) 2014 Vsevolod Velichko <torkvemada@sorokdva.net>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

#[macro_use]
extern crate uucore;

use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use uucore::fs::{canonicalize, CanonicalizeMode};

static NAME: &'static str = "relpath";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("h", "help", "Show help and exit");
    opts.optflag("V", "version", "Show version and exit");
    opts.optopt("d", "", "If any of FROM and TO is not subpath of DIR, output absolute path instead of relative", "DIR");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            show_error!("{}", f);
            show_usage(&opts);
            return 1
        }
    };

    if matches.opt_present("V") { version(); return 0 }
    if matches.opt_present("h") { show_usage(&opts); return 0 }

    if matches.free.is_empty() {
        show_error!("Missing operand: TO");
        println!("Try `{} --help` for more information.", NAME);
        return 1
    }

    let to = Path::new(&matches.free[0]);
    let from = if matches.free.len() > 1 {
        Path::new(&matches.free[1]).to_path_buf()
    } else {
        env::current_dir().unwrap()
    };
    let absto = canonicalize(to, CanonicalizeMode::Normal).unwrap();
    let absfrom = canonicalize(from, CanonicalizeMode::Normal).unwrap();

    if matches.opt_present("d") {
        let base = Path::new(&matches.opt_str("d").unwrap()).to_path_buf();
        let absbase = canonicalize(base, CanonicalizeMode::Normal).unwrap();
        if !absto.as_path().starts_with(absbase.as_path()) || !absfrom.as_path().starts_with(absbase.as_path()) {
            println!("{}", absto.display());
            return 0
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
    absfrom.components().skip(suffix_pos).map(|_| result.push("..")).last();
    absto.components().skip(suffix_pos).map(|x| result.push(x.as_ref())).last();

    println!("{}", result.display());
    0
}

fn version() {
    println!("{} {}", NAME, VERSION)
}

fn show_usage(opts: &getopts::Options) {
    version();
    println!("");
    println!("Usage:");
    println!("  {} [-d DIR] TO [FROM]", NAME);
    println!("  {} -V|--version", NAME);
    println!("  {} -h|--help", NAME);
    println!("");
    print!("{}", opts.usage(
            "Convert TO destination to the relative path from the FROM dir.\n\
            If FROM path is omitted, current working dir will be used.")
    );
}
