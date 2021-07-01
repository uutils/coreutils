// This file is part of the uutils coreutils package.
//
// (c) Alex Lyon <arcterus@mail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) Chmoder cmode fmode fperm fref ugoa RFILE RFILE's

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use uucore::fs::display_permissions_unix;
use uucore::libc::mode_t;
#[cfg(not(windows))]
use uucore::mode;
use uucore::InvalidEncodingHandling;
use walkdir::WalkDir;

static ABOUT: &str = "Change the mode of each FILE to MODE.
 With --reference, change the mode of each FILE to that of RFILE.";

mod options {
    pub const CHANGES: &str = "changes";
    pub const QUIET: &str = "quiet"; // visible_alias("silent")
    pub const VERBOSE: &str = "verbose";
    pub const NO_PRESERVE_ROOT: &str = "no-preserve-root";
    pub const PRESERVE_ROOT: &str = "preserve-root";
    pub const REFERENCE: &str = "RFILE";
    pub const RECURSIVE: &str = "recursive";
    pub const MODE: &str = "MODE";
    pub const FILE: &str = "FILE";
}

fn get_usage() -> String {
    format!(
        "{0} [OPTION]... MODE[,MODE]... FILE...
or: {0} [OPTION]... OCTAL-MODE FILE...
or: {0} [OPTION]... --reference=RFILE FILE...",
        executable!()
    )
}

fn get_long_usage() -> String {
    String::from("Each MODE is of the form '[ugoa]*([-+=]([rwxXst]*|[ugo]))+|[-+=]?[0-7]+'.")
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let mut args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    // Before we can parse 'args' with clap (and previously getopts),
    // a possible MODE prefix '-' needs to be removed (e.g. "chmod -x FILE").
    let mode_had_minus_prefix = strip_minus_from_mode(&mut args);

    let usage = get_usage();
    let after_help = get_long_usage();

    let matches = uu_app()
        .usage(&usage[..])
        .after_help(&after_help[..])
        .get_matches_from(args);

    let changes = matches.is_present(options::CHANGES);
    let quiet = matches.is_present(options::QUIET);
    let verbose = matches.is_present(options::VERBOSE);
    let preserve_root = matches.is_present(options::PRESERVE_ROOT);
    let recursive = matches.is_present(options::RECURSIVE);
    let fmode = matches
        .value_of(options::REFERENCE)
        .and_then(|fref| match fs::metadata(fref) {
            Ok(meta) => Some(meta.mode()),
            Err(err) => crash!(1, "cannot stat attributes of '{}': {}", fref, err),
        });
    let modes = matches.value_of(options::MODE).unwrap(); // should always be Some because required
    let cmode = if mode_had_minus_prefix {
        // clap parsing is finished, now put prefix back
        format!("-{}", modes)
    } else {
        modes.to_string()
    };
    let mut files: Vec<String> = matches
        .values_of(options::FILE)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();
    let cmode = if fmode.is_some() {
        // "--reference" and MODE are mutually exclusive
        // if "--reference" was used MODE needs to be interpreted as another FILE
        // it wasn't possible to implement this behavior directly with clap
        files.push(cmode);
        None
    } else {
        Some(cmode)
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
    match chmoder.chmod(files) {
        Ok(()) => {}
        Err(e) => return e,
    }

    0
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(util_name!())
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::CHANGES)
                .long(options::CHANGES)
                .short("c")
                .help("like verbose but report only when a change is made"),
        )
        .arg(
            Arg::with_name(options::QUIET)
                .long(options::QUIET)
                .visible_alias("silent")
                .short("f")
                .help("suppress most error messages"),
        )
        .arg(
            Arg::with_name(options::VERBOSE)
                .long(options::VERBOSE)
                .short("v")
                .help("output a diagnostic for every file processed"),
        )
        .arg(
            Arg::with_name(options::NO_PRESERVE_ROOT)
                .long(options::NO_PRESERVE_ROOT)
                .help("do not treat '/' specially (the default)"),
        )
        .arg(
            Arg::with_name(options::PRESERVE_ROOT)
                .long(options::PRESERVE_ROOT)
                .help("fail to operate recursively on '/'"),
        )
        .arg(
            Arg::with_name(options::RECURSIVE)
                .long(options::RECURSIVE)
                .short("R")
                .help("change files and directories recursively"),
        )
        .arg(
            Arg::with_name(options::REFERENCE)
                .long("reference")
                .takes_value(true)
                .help("use RFILE's mode instead of MODE values"),
        )
        .arg(
            Arg::with_name(options::MODE)
                .required_unless(options::REFERENCE)
                .takes_value(true),
            // It would be nice if clap could parse with delimiter, e.g. "g-x,u+x",
            // however .multiple(true) cannot be used here because FILE already needs that.
            // Only one positional argument with .multiple(true) set is allowed per command
        )
        .arg(
            Arg::with_name(options::FILE)
                .required_unless(options::MODE)
                .multiple(true),
        )
}

// Iterate 'args' and delete the first occurrence
// of a prefix '-' if it's associated with MODE
// e.g. "chmod -v -xw -R FILE" -> "chmod -v xw -R FILE"
pub fn strip_minus_from_mode(args: &mut Vec<String>) -> bool {
    for arg in args {
        if arg.starts_with('-') {
            if let Some(second) = arg.chars().nth(1) {
                match second {
                    'r' | 'w' | 'x' | 'X' | 's' | 't' | 'u' | 'g' | 'o' | '0'..='7' => {
                        // TODO: use strip_prefix() once minimum rust version reaches 1.45.0
                        *arg = arg[1..arg.len()].to_string();
                        return true;
                    }
                    _ => {}
                }
            }
        }
    }
    false
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
            if !file.exists() {
                if is_symlink(file) {
                    println!(
                        "failed to change mode of '{}' from 0000 (---------) to 0000 (---------)",
                        filename
                    );
                    if !self.quiet {
                        show_error!("cannot operate on dangling symlink '{}'", filename);
                    }
                } else {
                    show_error!("cannot access '{}': No such file or directory", filename);
                }
                return Err(1);
            }
            if self.recursive && self.preserve_root && filename == "/" {
                show_error!(
                    "it is dangerous to operate recursively on '{}'\nuse --no-preserve-root to override this failsafe",
                    filename
                );
                return Err(1);
            }
            if !self.recursive {
                r = self.chmod_file(file).and(r);
            } else {
                for entry in WalkDir::new(&filename).into_iter().filter_map(|e| e.ok()) {
                    let file = entry.path();
                    r = self.chmod_file(file).and(r);
                }
            }
        }
        r
    }

    #[cfg(windows)]
    fn chmod_file(&self, file: &Path) -> Result<(), i32> {
        // chmod is useless on Windows
        // it doesn't set any permissions at all
        // instead it just sets the readonly attribute on the file
        Err(0)
    }
    #[cfg(any(unix, target_os = "redox"))]
    fn chmod_file(&self, file: &Path) -> Result<(), i32> {
        let mut fperm = match fs::metadata(file) {
            Ok(meta) => meta.mode() & 0o7777,
            Err(err) => {
                if is_symlink(file) {
                    if self.verbose {
                        println!(
                            "neither symbolic link '{}' nor referent has been changed",
                            file.display()
                        );
                    }
                    return Ok(());
                } else if err.kind() == std::io::ErrorKind::PermissionDenied {
                    show_error!("'{}': Permission denied", file.display());
                } else {
                    show_error!("'{}': {}", file.display(), err);
                }
                return Err(1);
            }
        };
        match self.fmode {
            Some(mode) => self.change_file(fperm, mode, file)?,
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
                            self.change_file(fperm, mode, file)?;
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
    fn change_file(&self, fperm: u32, mode: u32, file: &Path) -> Result<(), i32> {
        if fperm == mode {
            if self.verbose && !self.changes {
                println!(
                    "mode of '{}' retained as {:04o} ({})",
                    file.display(),
                    fperm,
                    display_permissions_unix(fperm as mode_t, false),
                );
            }
            Ok(())
        } else if let Err(err) = fs::set_permissions(file, fs::Permissions::from_mode(mode)) {
            if !self.quiet {
                show_error!("{}", err);
            }
            if self.verbose {
                show_error!(
                    "failed to change mode of file '{}' from {:o} ({}) to {:o} ({})",
                    file.display(),
                    fperm,
                    display_permissions_unix(fperm as mode_t, false),
                    mode,
                    display_permissions_unix(mode as mode_t, false)
                );
            }
            Err(1)
        } else {
            if self.verbose || self.changes {
                show_error!(
                    "mode of '{}' changed from {:o} ({}) to {:o} ({})",
                    file.display(),
                    fperm,
                    display_permissions_unix(fperm as mode_t, false),
                    mode,
                    display_permissions_unix(mode as mode_t, false)
                );
            }
            Ok(())
        }
    }
}

pub fn is_symlink<P: AsRef<Path>>(path: P) -> bool {
    match fs::symlink_metadata(path) {
        Ok(m) => m.file_type().is_symlink(),
        Err(_) => false,
    }
}
