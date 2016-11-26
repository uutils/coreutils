#![crate_name = "uu_mktemp"]

// This file is part of the uutils coreutils package.
//
// (c) Sunrin SHIMURA
// Collaborator: Jian Zeng
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//

extern crate getopts;
extern crate tempfile;
extern crate rand;

#[macro_use]
extern crate uucore;

use std::env;
use std::io::Write;
use std::path::{PathBuf, is_separator};
use std::mem::forget;
use std::iter;

use rand::Rng;
use tempfile::NamedTempFileOptions;

mod tempdir;

static NAME: &'static str = "mktemp";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");
static DEFAULT_TEMPLATE: &'static str = "tmp.XXXXXXXXXX";


pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();
    opts.optflag("d", "directory", "Make a directory instead of a file");
    opts.optflag("u",
                 "dry-run",
                 "do not create anything; merely print a name (unsafe)");
    opts.optflag("q", "quiet", "Fail silently if an error occurs.");
    opts.optopt("", "suffix", "append SUFF to TEMPLATE; SUFF must not contain a path separator. This option is implied if TEMPLATE does not end with X.", "SUFF");
    opts.optopt("p", "tmpdir", "interpret TEMPLATE relative to DIR; if DIR is not specified, use $TMPDIR if set, else /tmp.  With this option, TEMPLATE  must  not  be  an  absolute name; unlike with -t, TEMPLATE may contain slashes, but mktemp creates only the final component", "DIR");
    // deprecated option of GNU coreutils
    //    opts.optflag("t", "", "Generate a template (using the supplied prefix and TMPDIR if set) to create a filename template");
    opts.optflag("", "help", "Print this help and exit");
    opts.optflag("", "version", "print the version and exit");


    // >> early return options
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f),
    };

    if matches.opt_present("help") {
        print_help(&opts);
        return 0;
    }
    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    if 1 < matches.free.len() {
        crash!(1, "Too many templates");
    }
    // <<

    let make_dir = matches.opt_present("directory");
    let dry_run = matches.opt_present("dry-run");
    let suffix_opt = matches.opt_str("suffix");
    let suppress_file_err = matches.opt_present("quiet");


    let template = if matches.free.is_empty() {
        DEFAULT_TEMPLATE
    } else {
        &matches.free[0][..]
    };

    let (prefix, rand, suffix) = match parse_template(template) {
        Some((p, r, s)) => {
            match suffix_opt {
                Some(suf) => {
                    if s == "" {
                        (p, r, suf)
                    } else {
                        crash!(1,
                               "Template should end with 'X' when you specify suffix option.")
                    }
                }
                None => (p, r, s.to_owned()),
            }
        }
        None => ("", 0, "".to_owned()),
    };

    if rand < 3 {
        crash!(1, "Too few 'X's in template")
    }

    if suffix.chars().any(is_separator) {
        crash!(1, "suffix cannot contain any path separators");
    }


    let tmpdir = match matches.opt_str("tmpdir") {
        Some(s) => {
            if PathBuf::from(prefix).is_absolute() {
                show_info!("invalid template, ‘{}’; with --tmpdir, it may not be absolute", template);
                return 1;
            }
            PathBuf::from(s)
        }
        None => env::temp_dir(),
    };

    if dry_run {
        dry_exec(tmpdir, prefix, rand, &suffix)
    } else {
        exec(tmpdir, prefix, rand, &suffix, make_dir, suppress_file_err)
    }

}

fn print_help(opts: &getopts::Options) {
    let usage = format!(" Create a temporary file or directory, safely, and print its name.
TEMPLATE must contain at least 3 consecutive 'X's in last component.
If TEMPLATE is not specified, use {}, and --tmpdir is implied",
                        DEFAULT_TEMPLATE);

    println!("{} {}", NAME, VERSION);
    println!("SYNOPSIS");
    println!("  {} [OPTION]... [FILE]", NAME);
    println!("Usage:");
    print!("{}", opts.usage(&usage[..]));
}

fn parse_template(temp: &str) -> Option<(&str, usize, &str)> {
    let right = match temp.rfind('X') {
        Some(r) => r + 1,
        None => return None,
    };
    let left = temp[..right].rfind(|c| c != 'X').map_or(0, |i| i + 1);
    let prefix = &temp[..left];
    let rand = right - left;
    let suffix = &temp[right..];
    Some((prefix, rand, suffix))
}

pub fn dry_exec(mut tmpdir: PathBuf, prefix: &str, rand: usize, suffix: &str) -> i32 {
    let len = prefix.len() + suffix.len() + rand;
    let mut buf = String::with_capacity(len);
    buf.push_str(prefix);
    buf.extend(iter::repeat('X').take(rand));
    buf.push_str(suffix);

    // Randomize.
    unsafe {
        // We guarantee utf8.
        let bytes = &mut buf.as_mut_vec()[prefix.len()..prefix.len() + rand];
        rand::thread_rng().fill_bytes(bytes);
        for byte in bytes.iter_mut() {
            *byte = match *byte % 62 {
                v @ 0...9 => (v + '0' as u8),
                v @ 10...35 => (v - 10 + 'a' as u8),
                v @ 36...61 => (v - 36 + 'A' as u8),
                _ => unreachable!(),
            }
        }
    }
    tmpdir.push(String::from(buf));
    println!("{}", tmpdir.display());
    0
}

fn exec(tmpdir: PathBuf, prefix: &str, rand: usize, suffix: &str, make_dir: bool, quiet: bool) -> i32 {
    if make_dir {
        match tempdir::new_in(&tmpdir, prefix, rand, suffix) {
            Ok(ref f) => {
                println!("{}", f);
                return 0;
            }
            Err(e) => {
                if !quiet {
                    show_info!("{}", e);
                }
                return 1;
            }
        }
    }

    let tmpfile = NamedTempFileOptions::new()
        .prefix(prefix)
        .rand_bytes(rand)
        .suffix(suffix)
        .create_in(tmpdir);

    let tmpfile = match tmpfile {
        Ok(f) => f,
        Err(e) => {
            if !quiet {
                show_info!("failed to create tempfile: {}", e);
            }
            return 1;
        }
    };

    let tmpname = tmpfile.path()
                         .to_string_lossy()
                         .to_string();

    println!("{}", tmpname);

    // CAUTION: Not to call `drop` of tmpfile, which removes the tempfile,
    // I call a dangerous function `forget`.
    forget(tmpfile);

    0
}
