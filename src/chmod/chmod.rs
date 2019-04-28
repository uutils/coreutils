#![crate_name = "uu_chmod"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alex Lyon <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[cfg(unix)]
extern crate libc;
extern crate walker;

#[macro_use]
extern crate uucore;

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use walker::Walker;
#[cfg(not(windows))]
use uucore::mode;
use uucore::fs::display_permissions_unix;

const NAME: &str = "chmod";
static SUMMARY: &str = "Change the mode of each FILE to MODE.
 With --reference, change the mode of each FILE to that of RFILE.";
static LONG_HELP: &str = "
 Each MODE is of the form '[ugoa]*([-+=]([rwxXst]*|[ugo]))+|[-+=]?[0-7]+'.
";

pub fn uumain(mut args: Vec<String>) -> i32 {
    let syntax = format!(
        "[OPTION]... MODE[,MODE]... FILE...
 {0} [OPTION]... OCTAL-MODE FILE...
 {0} [OPTION]... --reference=RFILE FILE...",
        NAME
    );
    let mut opts = new_coreopts!(&syntax, SUMMARY, LONG_HELP);
    opts.optflag("c", "changes", "like verbose but report only when a change is made")
        // TODO: support --silent (can be done using clap)
        .optflag("f", "quiet", "suppress most error messages")
        .optflag("v", "verbose", "output a diagnostic for every file processed")
        .optflag("", "no-preserve-root", "do not treat '/' specially (the default)")
        .optflag("", "preserve-root", "fail to operate recursively on '/'")
        .optopt("", "reference", "use RFILE's mode instead of MODE values", "RFILE")
        .optflag("R", "recursive", "change files and directories recursively");

    // sanitize input for - at beginning (e.g. chmod -x testfile). Remove
    // the option and save it for later, after parsing is finished.
    let negative_option = sanitize_input(&mut args);

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
        let fmode = matches
            .opt_str("reference")
            .and_then(|ref fref| match fs::metadata(fref) {
                Ok(meta) => Some(meta.mode()),
                Err(err) => crash!(1, "cannot stat attribues of '{}': {}", fref, err),
            });
        let cmode = if fmode.is_none() {
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
        let chmoder = Chmoder {
            changes,
            quiet,
            verbose,
            preserve_root,
            recursive,
            fmode,
            cmode,
        };
        match chmoder.chmod(matches.free) {
            Ok(()) => {}
            Err(e) => return e,
        }
    }

    0
}

fn sanitize_input(args: &mut Vec<String>) -> Option<String> {
    for i in 0..args.len() {
        let first = args[i].chars().nth(0).unwrap();
        if first != '-' {
            continue;
        }
        if let Some(second) = args[i].chars().nth(1) {
            match second {
                'r' | 'w' | 'x' | 'X' | 's' | 't' | 'u' | 'g' | 'o' | '0'...'7' => {
                    return Some(args.remove(i));
                }
                _ => {}
            }
        }
    }
    None
}

struct Chmoder {
    changes: bool,
    quiet: bool,
    verbose: bool,
    preserve_root: bool,
    recursive: bool,
    fmode: Option<u32>,
    cmode: Option<String>,
}

impl Chmoder {
    fn chmod(&self, files: Vec<String>) -> Result<(), i32> {
        let mut r = Ok(());

        for filename in &files {
            let filename = &filename[..];
            let file = Path::new(filename);
            if file.exists() {
                if file.is_dir() {
                    if !self.preserve_root || filename != "/" {
                        if self.recursive {
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
                            r = self.chmod(
                                walk_dir
                                    .filter_map(|x| match x {
                                        Ok(o) => match o.path().into_os_string().to_str() {
                                            Some(s) => Some(s.to_owned()),
                                            None => None,
                                        },
                                        Err(_) => None,
                                    })
                                    .collect(),
                            ).and(r);
                            r = self.chmod_file(&file, filename).and(r);
                        }
                    } else {
                        show_error!("could not change permissions of directory '{}'", filename);
                        r = Err(1);
                    }
                } else {
                    r = self.chmod_file(&file, filename).and(r);
                }
            } else {
                show_error!("no such file or directory '{}'", filename);
                r = Err(1);
            }
        }

        r
    }

    #[cfg(windows)]
    fn chmod_file(&self, file: &Path, name: &str) -> Result<(), i32> {
        // chmod is useless on Windows
        // it doesn't set any permissions at all
        // instead it just sets the readonly attribute on the file
        Err(0)
    }
    #[cfg(any(unix, target_os = "redox"))]
    fn chmod_file(&self, file: &Path, name: &str) -> Result<(), i32> {
        let mut fperm = match fs::metadata(name) {
            Ok(meta) => meta.mode() & 0o7777,
            Err(err) => {
                if !self.quiet {
                    show_error!("{}", err);
                }
                return Err(1);
            }
        };
        match self.fmode {
            Some(mode) => self.change_file(fperm, mode, file, name)?,
            None => {
                let cmode_unwrapped = self.cmode.clone().unwrap();
                for mode in cmode_unwrapped.split(',') {
                    // cmode is guaranteed to be Some in this case
                    let arr: &[char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
                    let result = if mode.contains(arr) {
                        mode::parse_numeric(fperm, mode)
                    } else {
                        mode::parse_symbolic(fperm, mode, file.is_dir())
                    };
                    match result {
                        Ok(mode) => {
                            self.change_file(fperm, mode, file, name)?;
                            fperm = mode;
                        }
                        Err(f) => {
                            if !self.quiet {
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

    #[cfg(unix)]
    fn change_file(&self, fperm: u32, mode: u32, file: &Path, path: &str) -> Result<(), i32> {
        if fperm == mode {
            if self.verbose && !self.changes {
                show_info!("mode of '{}' retained as {:o} ({})", file.display(), fperm, display_permissions_unix(fperm));
            }
            Ok(())
        } else if let Err(err) =
            fs::set_permissions(Path::new(path), fs::Permissions::from_mode(mode))
        {
            if !self.quiet {
                show_error!("{}", err);
            }
            if self.verbose {
                show_info!(
                    "failed to change mode of file '{}' from {:o} ({}) to {:o} ({})",
                    file.display(),
                    fperm,
                    display_permissions_unix(fperm),
                    mode,
                    display_permissions_unix(mode)
                );
            }
            Err(1)
        } else {
            if self.verbose || self.changes {
                show_info!(
                    "mode of '{}' changed from {:o} ({}) to {:o} ({})",
                    file.display(),
                    fperm,
                    display_permissions_unix(fperm),
                    mode,
                    display_permissions_unix(mode)
                );
            }
            Ok(())
        }
    }
}
