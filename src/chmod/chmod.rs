#![crate_name = "uu_chmod"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[cfg(unix)]
extern crate libc;
extern crate walker;

#[macro_use]
extern crate uucore;

use std::error::Error;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use walker::Walker;

const NAME: &'static str = "chmod";
static SUMMARY: &'static str = "Change the mode of each FILE to MODE.
 With --reference, change the mode of each FILE to that of RFILE."; 
static LONG_HELP: &'static str = "
 Each MODE is of the form '[ugoa]*([-+=]([rwxXst]*|[ugo]))+|[-+=]?[0-7]+'.
";

pub fn uumain(mut args: Vec<String>) -> i32 {
    let syntax = format!("[OPTION]... MODE[,MODE]... FILE...
 {0} [OPTION]... OCTAL-MODE FILE...
 {0} [OPTION]... --reference=RFILE FILE...", NAME);
    let mut opts = new_coreopts!(&syntax, SUMMARY, LONG_HELP);
    opts.optflag("c", "changes", "like verbose but report only when a change is made (unimplemented)")
        .optflag("f", "quiet", "suppress most error messages (unimplemented)") // TODO: support --silent
        .optflag("v", "verbose", "output a diagnostic for every file processed (unimplemented)")
        .optflag("", "no-preserve-root", "do not treat '/' specially (the default)")
        .optflag("", "preserve-root", "fail to operate recursively on '/'")
        .optopt("", "reference", "use RFILE's mode instead of MODE values", "RFILE")
        .optflag("R", "recursive", "change files and directories recursively");

    // sanitize input for - at beginning (e.g. chmod -x testfile). Remove
    // the option and save it for later, after parsing is finished.
    let mut negative_option = None;
    for i in 0..args.len() {
        if let Some(first) = args[i].chars().nth(0) {
            if first != '-' {
                continue;
            }

            if let Some(second) = args[i].chars().nth(1) {
                match second {
                    'r' | 'w' | 'x' | 'X' | 's' | 't' | 'u' | 'g' | 'o' | '0' ... '7' => {
                        negative_option = Some(args.remove(i));
                        break;
                    },
                    _ => {}
                }
            }
        }
    }

    let mut matches = opts.parse(args);
    if matches.free.is_empty() {
        show_error!("missing an argument");
        show_error!("for help, try '{} --help'", NAME);
        return 1;
    } else {
        let changes = matches.opt_present("changes");
        let quiet = matches.opt_present("quiet");
        let verbose = matches.opt_present("verbose");
        let preserve_root = matches.opt_present("preserve-root");
        let recursive = matches.opt_present("recursive");
        let fmode = matches.opt_str("reference").and_then(|ref fref| {
            match fs::metadata(fref) {
                Ok(meta) => Some(meta.mode()),
                Err(err) => crash!(1, "cannot stat attribues of '{}': {}", fref, err)
            }
        });
        let cmode =
            if fmode.is_none() {
                // If there was a negative option, now it's a good time to
                // use it.
                if negative_option.is_some() {
                    negative_option
                } else {
                    Some(matches.free.remove(0))
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

fn chmod(files: Vec<String>, changes: bool, quiet: bool, verbose: bool, preserve_root: bool, recursive: bool, fmode: Option<u32>, cmode: Option<&String>) -> Result<(), i32> {
    let mut r = Ok(());

    for filename in &files {
        let filename = &filename[..];
        let file = Path::new(filename);
        if file.exists() {
            if file.is_dir() {
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
                                                                Some(s) => Some(s.to_owned()),
                                                                None => None,
                                                            },
                                                            Err(_) => None,
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
fn chmod_file(file: &Path, name: &str, changes: bool, quiet: bool, verbose: bool, fmode: Option<u32>, cmode: Option<&String>) -> Result<(), i32> {
    // chmod is useless on Windows
    // it doesn't set any permissions at all
    // instead it just sets the readonly attribute on the file
    Err(0)
}
#[cfg(any(unix, target_os = "redox"))]
fn chmod_file(file: &Path, name: &str, changes: bool, quiet: bool, verbose: bool, fmode: Option<u32>, cmode: Option<&String>) -> Result<(), i32> {
    let mut fperm = match fs::metadata(name) {
        Ok(meta) => meta.mode() & 0o7777,
        Err(err) => {
            if !quiet {
                show_error!("{}", err);
            }
            return Err(1);
        }
    };
    match fmode {
        Some(mode) => try!(change_file(fperm, mode, file, name, verbose, changes, quiet)),
        None => {
            for mode in cmode.unwrap().split(',') {  // cmode is guaranteed to be Some in this case
                let arr: &[char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
                let result =
                    if mode.contains(arr) {
                        parse_numeric(fperm, mode)
                    } else {
                        parse_symbolic(fperm, mode, file)
                    };
                match result {
                    Ok(mode) => {
                        try!(change_file(fperm, mode, file, name, verbose, changes, quiet));
                        fperm = mode;
                    }
                    Err(f) => {
                        if !quiet {
                            show_error!("{}", f);
                        }
                        return Err(1);
                    }
                }
            }
        }
    }

    Ok(())
}

fn parse_numeric(fperm: u32, mut mode: &str) -> Result<u32, String> {
    let (op, pos) = try!(parse_op(mode, Some('=')));
    mode = mode[pos..].trim_left_matches('0');
    if mode.len() > 4 {
        Err(format!("mode is too large ({} > 7777)", mode))
    } else {
        match u32::from_str_radix(mode, 8) {
            Ok(change) => {
                Ok(match op {
                    '+' => fperm | change,
                    '-' => fperm & !change,
                    '=' => change,
                    _ => unreachable!()
                })
            }
            Err(err) => Err(err.description().to_owned())
        }
    }
}

fn parse_symbolic(mut fperm: u32, mut mode: &str, file: &Path) -> Result<u32, String> {
    #[cfg(unix)]
    use libc::umask;

    #[cfg(target_os = "redox")]
    unsafe fn umask(_mask: u32) -> u32 {
        // XXX Redox does not currently have umask
        0
    }

    let (mask, pos) = parse_levels(mode);
    if pos == mode.len() {
        return Err(format!("invalid mode ({})", mode));
    }
    let respect_umask = pos == 0;
    let last_umask = unsafe {
        umask(0)
    };
    mode = &mode[pos..];
    while mode.len() > 0 {
        let (op, pos) = try!(parse_op(mode, None));
        mode = &mode[pos..];
        let (mut srwx, pos) = parse_change(mode, fperm, file);
        if respect_umask {
            srwx &= !(last_umask as u32);
        }
        mode = &mode[pos..];
        match op {
            '+' => fperm |= srwx & mask,
            '-' => fperm &= !(srwx & mask),
            '=' => fperm = (fperm & !mask) | (srwx & mask),
            _ => unreachable!()
        }
    }
    unsafe {
        umask(last_umask);
    }
    Ok(fperm)
}

fn parse_levels(mode: &str) -> (u32, usize) {
    let mut mask = 0;
    let mut pos = 0;
    for ch in mode.chars() {
        mask |= match ch {
            'u' => 0o7700,
            'g' => 0o7070,
            'o' => 0o7007,
            'a' => 0o7777,
            _ => break
        };
        pos += 1;
    }
    if pos == 0 {
        mask = 0o7777;  // default to 'a'
    }
    (mask, pos)
}

fn parse_op(mode: &str, default: Option<char>) -> Result<(char, usize), String> {
    match mode.chars().next() {
        Some(ch) => match ch {
            '+' | '-' | '=' => Ok((ch, 1)),
            _ => match default {
                Some(ch) => Ok((ch, 0)),
                None => Err(format!("invalid operator (expected +, -, or =, but found {})", ch))
            }
        },
        None => Err("unexpected end of mode".to_owned())
    }
}

fn parse_change(mode: &str, fperm: u32, file: &Path) -> (u32, usize) {
    let mut srwx = fperm & 0o7000;
    let mut pos = 0;
    for ch in mode.chars() {
        match ch {
            'r' => srwx |= 0o444,
            'w' => srwx |= 0o222,
            'x' => srwx |= 0o111,
            'X' => {
                if file.is_dir() || (fperm & 0o0111) != 0 {
                    srwx |= 0o111
                }
            }
            's' => srwx |= 0o4000 | 0o2000,
            't' => srwx |= 0o1000,
            'u' => srwx = (fperm & 0o700) | ((fperm >> 3) & 0o070) | ((fperm >> 6) & 0o007),
            'g' => srwx = ((fperm << 3) & 0o700) | (fperm & 0o070) | ((fperm >> 3) & 0o007),
            'o' => srwx = ((fperm << 6) & 0o700) | ((fperm << 3) & 0o070) | (fperm & 0o007),
            _ => break
        };
        pos += 1;
    }
    if pos == 0 {
        srwx = 0;
    }
    (srwx, pos)
}

fn change_file(fperm: u32, mode: u32, file: &Path, path: &str, verbose: bool, changes: bool, quiet: bool) -> Result<(), i32> {
    if fperm == mode {
        if verbose && !changes {
            show_info!("mode of '{}' retained as {:o}", file.display(), fperm);
        }
        Ok(())
    } else if let Err(err) = fs::set_permissions(Path::new(path), fs::Permissions::from_mode(mode)) {
        if !quiet {
            show_error!("{}", err);
        }
        if verbose {
            show_info!("failed to change mode of file '{}' from {:o} to {:o}", file.display(), fperm, mode);
        }
        Err(1)
    } else {
        if verbose || changes {
            show_info!("mode of '{}' changed from {:o} to {:o}", file.display(), fperm, mode);
        }
        Ok(())
    }
}
