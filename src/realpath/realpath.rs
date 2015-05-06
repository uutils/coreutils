#![crate_name= "realpath"]
#![feature(file_type, path_ext, rustc_private)]

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

use getopts::{getopts, optflag, usage};
use std::fs::PathExt;
use std::io::Write;
use std::path::{Path, PathBuf};

#[path = "../common/util.rs"] #[macro_use] mod util;

static NAME: &'static str = "realpath";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> i32 {
    let program = &args[0];
    let options = [
        optflag("h", "help", "Show help and exit"),
        optflag("V", "version", "Show version and exit"),
        optflag("s", "strip", "Only strip '.' and '..' components, but don't resolve symbolic links"),
        optflag("z", "zero", "Separate output filenames with \\0 rather than newline"),
        optflag("q", "quiet", "Do not print warnings for invalid paths"),
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
        show_error!("Missing operand: FILENAME, at least one is required");
        println!("Try `{} --help` for more information.", program);
        return 1
    }

    let strip = opts.opt_present("s");
    let zero = opts.opt_present("z");
    let quiet = opts.opt_present("q");
    let mut retcode = 0;
    for path in opts.free.iter() {
        if !resolve_path(path, strip, zero, quiet) {
            retcode = 1
        };
    }
    retcode
}

fn resolve_path(path: &str, strip: bool, zero: bool, quiet: bool) -> bool {
    let p = Path::new(path).to_path_buf();
    let abs = p.canonicalize().unwrap();

    if strip {
        if zero {
            print!("{}\0", abs.display());
        } else {
            println!("{}", abs.display())
        }
        return true
    }

    let mut result = PathBuf::new();
    let mut links_left = 256;

    for part in abs.components() {
        result.push(part.as_ref());
        loop {
            if links_left == 0 {
                if !quiet { show_error!("Too many symbolic links: {}", path) };
                return false
            }
            match result.as_path().metadata() {
                Err(_) => break,
                Ok(ref m) if !m.file_type().is_symlink() => break,
                Ok(_) => {
                    links_left -= 1;
                    match result.as_path().read_link() {
                        Ok(x) => {
                            result.pop();
                            result.push(x.as_path());
                        },
                        _ => {
                            if !quiet {
                                show_error!("Invalid path: {}", path)
                            };
                            return false
                        },
                    }
                }
            }
        }
    }

    if zero {
        print!("{}\0", result.display());
    } else {
        println!("{}", result.display());
    }

    true
}

fn version() {
    println!("{} v{}", NAME, VERSION)
}

fn show_usage(program: &str, options: &[getopts::OptGroup]) {
    version();
    println!("Usage:");
    println!("  {} [-s|--strip] [-z|--zero] FILENAME…", program);
    println!("  {} -V|--version", program);
    println!("  {} -h|--help", program);
    println!("");
    print!("{}", usage(
            "Convert each FILENAME to the absolute path.\n\
            All the symbolic links will be resolved, resulting path will contain no special components like '.' or '..'.\n\
            Each path component must exist or resolution will fail and non-zero exit status returned.\n\
            Each resolved FILENAME will be written to the standard output, one per line.", options)
    );
}
