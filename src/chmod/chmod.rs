#![crate_name = "chmod"]
#![feature(collections, core, io, libc, path, rustc_private, std_misc)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![allow(unused_variables)]  // only necessary while the TODOs still exist
#![feature(plugin)]

extern crate getopts;
extern crate libc;
extern crate regex;
#[plugin] #[no_link] extern crate regex_macros;

use std::ffi::CString;
use std::old_io::fs;
use std::old_io::fs::PathExtensions;
use std::old_io::IoError;
use std::mem;
use std::num::from_str_radix;
use regex::Regex;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

const NAME: &'static str = "chmod";
const VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> isize {
    let program = args[0].clone();

    let opts = [
        getopts::optflag("c", "changes", "like verbose but report only when a change is made (unimplemented)"),
        getopts::optflag("f", "quiet", "suppress most error messages (unimplemented)"), // TODO: support --silent
        getopts::optflag("v", "verbose", "output a diagnostic for every file processed (unimplemented)"),
        getopts::optflag("", "no-preserve-root", "do not treat '/' specially (the default)"),
        getopts::optflag("", "preserve-root", "fail to operate recursively on '/'"),
        getopts::optflagopt("", "reference", "use RFILE's mode instead of MODE values", "RFILE"),
        getopts::optflag("R", "recursive", "change files and directories recursively"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];
    // TODO: sanitize input for - at beginning (e.g. chmod -x testfile).  Solution is to add a to -x, making a-x
    let mut matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => {
            crash!(1, "{}", f)
        }
    };
    if matches.opt_present("help") {
        println!("{name} v{version}

Usage:
  {program} [OPTION]... MODE[,MODE]... FILE...
  {program} [OPTION]... OCTAL-MODE FILE...
  {program} [OPTION]... --reference=RFILE FILE...

{usage}
Each MODE is of the form '[ugoa]*([-+=]([rwxXst]*|[ugo]))+|[-+=]?[0-7]+'.",
               name = NAME, version = VERSION, program = program,
               usage = getopts::usage("Change the mode of each FILE to MODE. \
                                       With --reference, change the mode of \
                                       each FILE to that of RFILE.", &opts));
    } else if matches.opt_present("version") {
        println!("{} v{}", NAME, VERSION);
    } else if matches.free.is_empty() && matches.opt_present("reference") || matches.free.len() < 2 {
        show_error!("missing an argument");
        show_error!("for help, try '{} --help'", program);
        return 1;
    } else {
        let changes = matches.opt_present("changes");
        let quiet = matches.opt_present("quiet");
        let verbose = matches.opt_present("verbose");
        let preserve_root = matches.opt_present("preserve-root");
        let recursive = matches.opt_present("recursive");
        let fmode = matches.opt_str("reference").and_then(|fref| {
            let mut stat = unsafe { mem::uninitialized() };
            let statres = unsafe { libc::stat(fref.as_slice().as_ptr() as *const i8, &mut stat as *mut libc::stat) };
            if statres == 0 {
                Some(stat.st_mode)
            } else {
                crash!(1, "{}", IoError::last_error())
            }
        });
        let cmode =
            if fmode.is_none() {
                let mode = matches.free.remove(0);
                match verify_mode(mode.as_slice()) {
                    Ok(_) => Some(mode),
                    Err(f) => {
                        show_error!("{}", f);
                        return 1;
                    }
                }
            } else {
                None
            };
        match chmod(matches.free, changes, quiet, verbose, preserve_root,
                    recursive, fmode, cmode.as_ref()) {
            Ok(()) => {}
            Err(e) => return e
        }
    }

    0
}

#[cfg(unix)]
#[inline]
fn verify_mode(modes: &str) -> Result<(), String> {
    static REGEXP: regex::Regex = regex!(r"^[ugoa]*([-+=]([rwxXst]*|[ugo]))+|[-+=]?[0-7]+$");
    for mode in modes.split(',') {
        if !REGEXP.is_match(mode) {
            return Err(format!("invalid mode '{}'", mode));
        }
    }
    Ok(())
}

#[cfg(windows)]
#[inline]
// XXX: THIS IS NOT TESTED!!!
fn verify_mode(mode: &str) -> Result<(), String> {
    static REGEXP: regex::Regex = regex!(r"^[ugoa]*(?:[-+=](?:([rwxXst]*)|[ugo]))+|[-+=]?([0-7]+)$");
    for mode in modes.split(',') {
        match REGEXP.captures(mode) {
            Some(cap) => {
                let symbols = cap.at(1);
                let numbers = cap.at(2);
                if symbols.contains("s") || symbols.contains("t") {
                    return Err("The 's' and 't' modes are not supported on Windows".to_string());
                } else if numbers.len() >= 4 && numbers.slice_to(num_len - 3).find(|ch| ch != '0').is_some() {
                    return Err("Setuid, setgid, and sticky modes are not supported on Windows".to_string());
                }
            }
            None => return Err(format!("invalid mode '{}'", mode))
        }
    }
    Ok(())
}

fn chmod(files: Vec<String>, changes: bool, quiet: bool, verbose: bool, preserve_root: bool, recursive: bool, fmode: Option<libc::mode_t>, cmode: Option<&String>) -> Result<(), isize> {
    let mut r = Ok(());

    for filename in files.iter() {
        let filename = filename.as_slice();
        let file = Path::new(filename);
        if file.exists() {
            if file.is_dir() {
                if !preserve_root || filename != "/" {
                    if recursive {
                        let walk_dir = match fs::walk_dir(&file) {
                            Ok(m) => m,
                            Err(f) => {
                                crash!(1, "{}", f.to_string());
                            }
                        };
                        r = chmod(walk_dir.map(|x| x.as_str().unwrap().to_string()).collect(), changes, quiet, verbose, preserve_root, recursive, fmode, cmode).and(r);
                        r = chmod_file(&file, filename, changes, quiet, verbose, fmode, cmode).and(r);
                    }
                } else {
                    show_error!("could not change permissions of directory '{}'",
                                filename);
                    r = Err(1);
                }
            } else {
                r = chmod_file(&file, filename.as_slice(), changes, quiet, verbose, fmode, cmode).and(r);
            }
        } else {
            show_error!("no such file or directory '{}'", filename);
            r = Err(1);
        }
    }

    r
}

fn chmod_file(file: &Path, name: &str, changes: bool, quiet: bool, verbose: bool, fmode: Option<libc::mode_t>, cmode: Option<&String>) -> Result<(), isize> {
    let path = CString::from_slice(name.as_bytes());
    match fmode {
        Some(mode) => {
            if unsafe { libc::chmod(path.as_ptr(), mode) } == 0 {
                // TODO: handle changes, quiet, and verbose
            } else {
                show_error!("{}", IoError::last_error());
                return Err(1);
            }
        }
        None => {
            // TODO: make the regex processing occur earlier (i.e. once in the main function)
            static REGEXP: regex::Regex = regex!(r"^(([ugoa]*)((?:[-+=](?:[rwxXst]*|[ugo]))+))|([-+=]?[0-7]+)$");
            let mut stat = unsafe { mem::uninitialized() };
            let statres = unsafe { libc::stat(path.as_ptr(), &mut stat as *mut libc::stat) };
            let mut fperm =
                if statres == 0 {
                    stat.st_mode
                } else {
                    show_error!("{}", IoError::last_error());
                    return Err(1);
                };
            for mode in cmode.unwrap().as_slice().split(',') {  // cmode is guaranteed to be Some in this case
                let cap = REGEXP.captures(mode).unwrap();  // mode was verified earlier, so this is safe
                if cap.at(1).unwrap() != "" {
                    // symbolic
                    let mut levels = cap.at(2).unwrap();
                    if levels.len() == 0 {
                        levels = "a";
                    }
                    let change_str = cap.at(3).unwrap().to_string() + "+";
                    let change = change_str.as_slice();
                    let mut action = change.char_at(0);
                    let mut rwx = 0;
                    let mut special = 0;
                    let mut special_changed = false;
                    for ch in change[1..].chars() {
                        match ch {
                            '+' | '-' | '=' => {
                                for level in levels.chars() {
                                    let (rwx, mask) = match level {
                                        'u' => (rwx << 6, 0o7077),
                                        'g' => (rwx << 3, 0o7707),
                                        'o' => (rwx, 0o7770),
                                        'a' => ((rwx << 6) | (rwx << 3) | rwx, 0o7000),
                                        _ => unreachable!()
                                    };
                                    match action {
                                        '+' => fperm |= rwx,
                                        '-' => fperm &= !rwx,
                                        '=' => fperm = (fperm & mask) | rwx,
                                        _ => unreachable!()
                                    }
                                }
                                if special_changed {
                                    match action {
                                        '+' => fperm |= special,
                                        '-' => fperm &= !special,
                                        '=' => fperm &= special | 0o0777,
                                        _ => unreachable!()
                                    }
                                }
                                action = ch;
                                rwx = 0;
                                special = 0;
                                special_changed = false;
                            }
                            'r' => rwx |= 0o004,
                            'w' => rwx |= 0o002,
                            'x' => rwx |= 0o001,
                            'X' => {
                                if file.is_dir() || (fperm & 0o0111) != 0 {
                                    rwx |= 0o001;
                                }
                            }
                            's' => {
                                special |= 0o4000 | 0o2000;
                                special_changed = true;
                            }
                            't' => {
                                special |= 0o1000;
                                special_changed = true;
                            }
                            'u' => rwx = (fperm >> 6) & 0o007,
                            'g' => rwx = (fperm >> 3) & 0o007,
                            'o' => rwx = (fperm >> 0) & 0o007,
                            _ => unreachable!()
                        }
                    }
                } else {
                    // numeric
                    let change = cap.at(4).unwrap();
                    let ch = change.char_at(0);
                    let (action, slice) = match ch {
                        '+' | '-' | '=' => (ch, &change[1..]),
                        _ => ('=', change)
                    };
                    let mode = from_str_radix::<u32>(slice, 8).unwrap() as libc::mode_t;  // already verified
                    match action {
                        '+' => fperm |= mode,
                        '-' => fperm &= !mode,
                        '=' => fperm = mode,
                        _ => unreachable!()
                    }
                }
                if unsafe { libc::chmod(path.as_ptr(), fperm) } == 0 {
                    // TODO: see above
                } else {
                    show_error!("{}", IoError::last_error());
                    return Err(1);
                }
            }
        }
    }

    Ok(())
}
