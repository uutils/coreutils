#![crate_name = "relpath"]
#![feature(path_ext, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) 2014 Vsevolod Velichko <torkvemada@sorokdva.net>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

use getopts::{getopts, optflag, optopt, usage};
use std::env;
use std::fs::PathExt;
use std::io::Write;
use std::path::{Path, PathBuf};

#[path = "../common/util.rs"] #[macro_use] mod util;

static NAME: &'static str = "relpath";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> i32 {
    let program = &args[0];
    let options = [
        optflag("h", "help", "Show help and exit"),
        optflag("V", "version", "Show version and exit"),
        optopt("d", "", "If any of FROM and TO is not subpath of DIR, output absolute path instead of relative", "DIR"),
    ];

    let opts = match getopts(&args[1..], &options) {
        Ok(m) => m,
        Err(f) => {
            show_error!("{}", f);
            show_usage(program, &options);
            return 1
        }
    };

    if opts.opt_present("V") { version(); return 0 }
    if opts.opt_present("h") { show_usage(program, &options); return 0 }

    if opts.free.len() == 0 {
        show_error!("Missing operand: TO");
        println!("Try `{} --help` for more information.", program);
        return 1
    }

    let to = Path::new(&opts.free[0]);
    let from = if opts.free.len() > 1 {
        Path::new(&opts.free[1]).to_path_buf()
    } else {
        env::current_dir().unwrap()
    };
    let absto = to.canonicalize().unwrap();
    let absfrom = from.canonicalize().unwrap();

    if opts.opt_present("d") {
        let base = Path::new(&opts.opt_str("d").unwrap()).to_path_buf();
        let absbase = base.canonicalize().unwrap();
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
    println!("{} v{}", NAME, VERSION)
}

fn show_usage(program: &str, options: &[getopts::OptGroup]) {
    version();
    println!("Usage:");
    println!("  {} [-d DIR] TO [FROM]", program);
    println!("  {} -V|--version", program);
    println!("  {} -h|--help", program);
    println!("");
    print!("{}", usage(
            "Convert TO destination to the relative path from the FROM dir.\n\
            If FROM path is omitted, current working dir will be used.", options)
    );
}
