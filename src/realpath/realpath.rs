#![crate_name= "uu_realpath"]

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

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use uucore::fs::{canonicalize, CanonicalizeMode};

static NAME: &'static str = "realpath";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("h", "help", "Show help and exit");
    opts.optflag("V", "version", "Show version and exit");
    opts.optflag("s", "strip", "Only strip '.' and '..' components, but don't resolve symbolic links");
    opts.optflag("z", "zero", "Separate output filenames with \\0 rather than newline");
    opts.optflag("q", "quiet", "Do not print warnings for invalid paths");

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
        show_error!("Missing operand: FILENAME, at least one is required");
        println!("Try `{} --help` for more information.", NAME);
        return 1
    }

    let strip = matches.opt_present("s");
    let zero = matches.opt_present("z");
    let quiet = matches.opt_present("q");
    let mut retcode = 0;
    for path in &matches.free {
        if !resolve_path(path, strip, zero, quiet) {
            retcode = 1
        };
    }
    retcode
}

fn resolve_path(path: &str, strip: bool, zero: bool, quiet: bool) -> bool {
    let p = Path::new(path).to_path_buf();
    let abs = canonicalize(p, CanonicalizeMode::Normal).unwrap();

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
            match fs::metadata(result.as_path()) {
                Err(_) => break,
                Ok(ref m) if !m.file_type().is_symlink() => break,
                Ok(_) => {
                    links_left -= 1;
                    match fs::read_link(result.as_path()) {
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
    println!("{} {}", NAME, VERSION)
}

fn show_usage(opts: &getopts::Options) {
    version();
    println!("");
    println!("Usage:");
    println!("  {} [-s|--strip] [-z|--zero] FILENAME...", NAME);
    println!("  {} -V|--version", NAME);
    println!("  {} -h|--help", NAME);
    println!("");
    print!("{}", opts.usage(
            "Convert each FILENAME to the absolute path.\n\
            All the symbolic links will be resolved, resulting path will contain no special components like '.' or '..'.\n\
            Each path component must exist or resolution will fail and non-zero exit status returned.\n\
            Each resolved FILENAME will be written to the standard output, one per line.")
    );
}
