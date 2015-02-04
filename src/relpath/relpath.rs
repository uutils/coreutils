#![crate_name = "relpath"]
#![feature(collections, core, libc, os, path, rustc_private)]

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

use getopts::{optflag, optopt, getopts, usage};

#[path = "../common/util.rs"] #[macro_use] mod util;

static NAME: &'static str = "relpath";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> isize {
    let program = &args[0];
    let options = [
        optflag("h", "help", "Show help and exit"),
        optflag("V", "version", "Show version and exit"),
        optopt("d", "", "If any of FROM and TO is not subpath of DIR, output absolute path instead of relative", "DIR"),
    ];

    let opts = match getopts(args.tail(), &options) {
        Ok(m) => m,
        Err(f) => {
            show_error!("{}", f);
            show_usage(program.as_slice(), &options);
            return 1
        }
    };

    if opts.opt_present("V") { version(); return 0 }
    if opts.opt_present("h") { show_usage(program.as_slice(), &options); return 0 }

    if opts.free.len() == 0 {
        show_error!("Missing operand: TO");
        println!("Try `{} --help` for more information.", program.as_slice());
        return 1
    }

    let to = Path::new(opts.free[0].as_slice());
    let from = if opts.free.len() > 1 {
        Path::new(opts.free[1].as_slice())
    } else {
        std::os::getcwd().unwrap()
    };
    let absto = std::os::make_absolute(&to).unwrap();
    let absfrom = std::os::make_absolute(&from).unwrap();

    if opts.opt_present("d") {
        let base = Path::new(opts.opt_str("d").unwrap());
        let absbase = std::os::make_absolute(&base).unwrap();
        if !absbase.is_ancestor_of(&absto) || !absbase.is_ancestor_of(&absfrom) {
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

    let mut result = Path::new("");
    absfrom.components().skip(suffix_pos).map(|_| result.push("..")).last();
    absto.components().skip(suffix_pos).map(|x| result.push(x)).last();

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
