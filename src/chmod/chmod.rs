#![crate_name = "chmod"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![allow(unused_variables)]  // only necessary while the TODOs still exist

extern crate aho_corasick;
extern crate getopts;
extern crate libc;
extern crate memchr;
extern crate regex;
extern crate regex_syntax;
extern crate walker;

use getopts::Options;
use regex::Regex;
use std::ffi::CString;
use std::io::{Error, Write};
use std::mem;
use std::path::Path;
use walker::Walker;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

#[path = "../common/filesystem.rs"]
mod filesystem;

use filesystem::UUPathExt;

const NAME: &'static str = "chmod";
const VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();
    opts.optflag("c", "changes", "like verbose but report only when a change is made (unimplemented)");
    opts.optflag("f", "quiet", "suppress most error messages (unimplemented)"); // TODO: support --silent
    opts.optflag("v", "verbose", "output a diagnostic for every file processed (unimplemented)");
    opts.optflag("", "no-preserve-root", "do not treat '/' specially (the default)");
    opts.optflag("", "preserve-root", "fail to operate recursively on '/'");
    opts.optflagopt("", "reference", "use RFILE's mode instead of MODE values", "RFILE");
    opts.optflag("R", "recursive", "change files and directories recursively");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");
    // TODO: sanitize input for - at beginning (e.g. chmod -x testfile).  Solution is to add a to -x, making a-x
    let mut matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => { crash!(1, "{}", f) }
    };
    if matches.opt_present("help") {
        let msg = format!("{name} {version}

Usage:
  {program} [OPTION]... MODE[,MODE]... FILE...
  {program} [OPTION]... OCTAL-MODE FILE...
  {program} [OPTION]... --reference=RFILE FILE...

Change the mode of each FILE to MODE.
With --reference, change the mode of each FILE to that of RFILE.
Each MODE is of the form '[ugoa]*([-+=]([rwxXst]*|[ugo]))+|[-+=]?[0-7]+'.",
            name = NAME, version = VERSION, program = NAME);

        print!("{}", opts.usage(&msg));
        return 0;
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
    } else if matches.free.is_empty() && matches.opt_present("reference") || matches.free.len() < 2 {
        show_error!("missing an argument");
        show_error!("for help, try '{} --help'", NAME);
        return 1;
    } else {
        let changes = matches.opt_present("changes");
        let quiet = matches.opt_present("quiet");
        let verbose = matches.opt_present("verbose");
        let preserve_root = matches.opt_present("preserve-root");
        let recursive = matches.opt_present("recursive");
        let fmode = matches.opt_str("reference").and_then(|fref| {
            let mut stat : libc::stat = unsafe { mem::uninitialized() };
            let statres = unsafe { libc::stat(fref.as_ptr() as *const i8, &mut stat as *mut libc::stat) };
            if statres == 0 {
                Some(stat.st_mode)
            } else {
                crash!(1, "{}", Error::last_os_error())
            }
        });
        let cmode =
            if fmode.is_none() {
                let mode = matches.free.remove(0);
                match verify_mode(&mode[..]) {
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
    let re: regex::Regex = Regex::new(r"^[ugoa]*([-+=]([rwxXst]*|[ugo]))+|[-+=]?[0-7]+$").unwrap();
    for mode in modes.split(',') {
        if !re.is_match(mode) {
            return Err(format!("invalid mode '{}'", mode));
        }
    }
    Ok(())
}

#[cfg(windows)]
#[inline]
// XXX: THIS IS NOT TESTED!!!
fn verify_mode(modes: &str) -> Result<(), String> {
    let re: regex::Regex = Regex::new(r"^[ugoa]*(?:[-+=](?:([rwxXst]*)|[ugo]))+|[-+=]?([0-7]+)$").unwrap();
    for mode in modes.split(',') {
        match re.captures(mode) {
            Some(cap) => {
                let symbols = cap.at(1).unwrap();
                let numbers = cap.at(2).unwrap();
                if symbols.contains("s") || symbols.contains("t") {
                    return Err("The 's' and 't' modes are not supported on Windows".into());
                } else if numbers.len() >= 4 && numbers[..numbers.len() - 3].find(|ch| ch != '0').is_some() {
                    return Err("Setuid, setgid, and sticky modes are not supported on Windows".into());
                }
            }
            None => return Err(format!("invalid mode '{}'", mode))
        }
    }
    Ok(())
}

fn chmod(files: Vec<String>, changes: bool, quiet: bool, verbose: bool, preserve_root: bool, recursive: bool, fmode: Option<libc::mode_t>, cmode: Option<&String>) -> Result<(), i32> {
    let mut r = Ok(());

    for filename in files.iter() {
        let filename = &filename[..];
        let file = Path::new(filename);
        if file.uu_exists() {
            if file.uu_is_dir() {
                if !preserve_root || filename != "/" {
                    if recursive {
                        let walk_dir = match Walker::new(&file) {
                            Ok(m) => m,
                            Err(f) => {
                                crash!(1, "{}", f.to_string());
                            }
                        };
                        // XXX: here (and elsewhere) we see that this impl will have issues
                        // with non-UTF-8 filenames. Using OsString won't fix this because
                        // on Windows OsStrings cannot be built out of non-UTF-8 chars. One
                        // possible fix is to use CStrings rather than Strings in the args
                        // to chmod() and chmod_file().
                        r = chmod(walk_dir.filter_map(|x| match x {
                                                            Ok(o) => match o.path().into_os_string().to_str() {
                                                                Some(s) => Some(s.to_string()),
                                                                None => None,
                                                            },
                                                            Err(e) => None,
                                                          }).collect(),
                                  changes, quiet, verbose, preserve_root, recursive, fmode, cmode).and(r);
                        r = chmod_file(&file, filename, changes, quiet, verbose, fmode, cmode).and(r);
                    }
                } else {
                    show_error!("could not change permissions of directory '{}'",
                                filename);
                    r = Err(1);
                }
            } else {
                r = chmod_file(&file, filename, changes, quiet, verbose, fmode, cmode).and(r);
            }
        } else {
            show_error!("no such file or directory '{}'", filename);
            r = Err(1);
        }
    }

    r
}

#[cfg(windows)]
fn chmod_file(file: &Path, name: &str, changes: bool, quiet: bool, verbose: bool, fmode: Option<libc::mode_t>, cmode: Option<&String>) -> Result<(), i32> {
    // chmod is useless on Windows
    // it doesn't set any permissions at all
    // instead it just sets the readonly attribute on the file
    Err(0)
}
#[cfg(unix)]
fn chmod_file(file: &Path, name: &str, changes: bool, quiet: bool, verbose: bool, fmode: Option<libc::mode_t>, cmode: Option<&String>) -> Result<(), i32> {
    let path = CString::new(name).unwrap_or_else(|e| panic!("{}", e));
    match fmode {
        Some(mode) => {
            if unsafe { libc::chmod(path.as_ptr(), mode) } == 0 {
                // TODO: handle changes, quiet, and verbose
            } else {
                show_error!("{}", Error::last_os_error());
                return Err(1);
            }
        }
        None => {
            // TODO: make the regex processing occur earlier (i.e. once in the main function)
            let re: regex::Regex = Regex::new(r"^(([ugoa]*)((?:[-+=](?:[rwxXst]*|[ugo]))+))|([-+=]?[0-7]+)$").unwrap();
            let mut stat: libc::stat = unsafe { mem::uninitialized() };
            let statres = unsafe { libc::stat(path.as_ptr(), &mut stat as *mut libc::stat) };
            let mut fperm =
                if statres == 0 {
                    stat.st_mode
                } else {
                    show_error!("{}", Error::last_os_error());
                    return Err(1);
                };
            for mode in cmode.unwrap().split(',') {  // cmode is guaranteed to be Some in this case
                let cap = re.captures(mode).unwrap();  // mode was verified earlier, so this is safe
                if match cap.at(1) {
                    Some("") | None => false,
                    _ => true,
                } {
                    // symbolic
                    let mut levels = cap.at(2).unwrap();
                    if levels.len() == 0 {
                        levels = "a";
                    }
                    let change = cap.at(3).unwrap().to_string() + "+";
                    let mut action = change.chars().next().unwrap();
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
                                if file.uu_is_dir() || (fperm & 0o0111) != 0 {
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
                    let ch = change.chars().next().unwrap();
                    let (action, slice) = match ch {
                        '+' | '-' | '=' => (ch, &change[1..]),
                        _ => ('=', change)
                    };
                    let mode = u32::from_str_radix(slice, 8).unwrap() as libc::mode_t;  // already verified
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
                    show_error!("{}", Error::last_os_error());
                    return Err(1);
                }
            }
        }
    }

    Ok(())
}
