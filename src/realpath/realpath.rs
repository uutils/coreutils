#![crate_name= "realpath"]
#![feature(collections, core, io, libc, os, path, rustc_private)]

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

use getopts::{optflag, getopts, usage};

#[path = "../common/util.rs"] #[macro_use] mod util;

static NAME: &'static str = "realpath";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> isize {
    let program = &args[0];
    let options = [
        optflag("h", "help", "Show help and exit"),
        optflag("V", "version", "Show version and exit"),
        optflag("s", "strip", "Only strip '.' and '..' components, but don't resolve symbolic links"),
        optflag("z", "zero", "Separate output filenames with \\0 rather than newline"),
        optflag("q", "quiet", "Do not print warnings for invalid paths"),
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
        show_error!("Missing operand: FILENAME, at least one is required");
        println!("Try `{} --help` for more information.", program.as_slice());
        return 1
    }

    let strip = opts.opt_present("s");
    let zero = opts.opt_present("z");
    let quiet = opts.opt_present("q");
    let mut retcode = 0;
    for path in opts.free.iter() {
        if !resolve_path(path.as_slice(), strip, zero, quiet) {
            retcode = 1
        };
    }
    retcode
}

fn resolve_path(path: &str, strip: bool, zero: bool, quiet: bool) -> bool {
    let p = Path::new(path);
    let abs = std::os::make_absolute(&p).unwrap();

    if strip {
        if zero {
            print!("{}\0", abs.display());
        } else {
            println!("{}", abs.display())
        }
        return true
    }

    let mut result = match abs.root_path() {
        None => crash!(2, "Broken path parse! Report to developers: {}", path),
        Some(x) => x,
    };

    let mut links_left = 256is;

    for part in abs.components() {
        result.push(part);
        loop {
            if links_left == 0 {
                if !quiet { show_error!("Too many symbolic links: {}", path) };
                return false
            }
            match std::old_io::fs::lstat(&result) {
                Err(_) => break,
                Ok(ref s) if s.kind != std::old_io::FileType::Symlink => break,
                Ok(_) => {
                    links_left -= 1;
                    match std::old_io::fs::readlink(&result) {
                        Ok(x) => {
                            result.pop();
                            result.push(x);
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
