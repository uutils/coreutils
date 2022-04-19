// This file is part of the uutils coreutils package.
//
// (c) Alex Lyon <arcterus@mail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) Chmoder cmode fmode fperm fref ugoa RFILE RFILE's

use clap::{crate_version, Arg, Command};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{ExitCode, UResult, USimpleError, UUsageError};
use uucore::fs::display_permissions_unix;
use uucore::libc::mode_t;
#[cfg(not(windows))]
use uucore::mode;
use uucore::{format_usage, show_error, InvalidEncodingHandling};

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

const USAGE: &str = "\
    {} [OPTION]... MODE[,MODE]... FILE...
    {} [OPTION]... OCTAL-MODE FILE...
    {} [OPTION]... --reference=RFILE FILE...";

fn get_long_usage() -> String {
    String::from("Each MODE is of the form '[ugoa]*([-+=]([rwxXst]*|[ugo]))+|[-+=]?[0-7]+'.")
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let mut args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    // Before we can parse 'args' with clap (and previously getopts),
    // a possible MODE prefix '-' needs to be removed (e.g. "chmod -x FILE").
    let mode_had_minus_prefix = mode::strip_minus_from_mode(&mut args);

    let after_help = get_long_usage();

    let matches = uu_app().after_help(&after_help[..]).get_matches_from(args);

    let changes = matches.is_present(options::CHANGES);
    let quiet = matches.is_present(options::QUIET);
    let verbose = matches.is_present(options::VERBOSE);
    let preserve_root = matches.is_present(options::PRESERVE_ROOT);
    let recursive = matches.is_present(options::RECURSIVE);
    let fmode = match matches.value_of(options::REFERENCE) {
        Some(fref) => match fs::metadata(fref) {
            Ok(meta) => Some(meta.mode()),
            Err(err) => {
                return Err(USimpleError::new(
                    1,
                    format!("cannot stat attributes of {}: {}", fref.quote(), err),
                ))
            }
        },
        None => None,
    };
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

    if files.is_empty() {
        return Err(UUsageError::new(1, "missing operand".to_string()));
    }

    let chmoder = Chmoder {
        changes,
        quiet,
        verbose,
        preserve_root,
        recursive,
        fmode,
        cmode,
    };

    chmoder.chmod(&files)
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::CHANGES)
                .long(options::CHANGES)
                .short('c')
                .help("like verbose but report only when a change is made"),
        )
        .arg(
            Arg::new(options::QUIET)
                .long(options::QUIET)
                .visible_alias("silent")
                .short('f')
                .help("suppress most error messages"),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .long(options::VERBOSE)
                .short('v')
                .help("output a diagnostic for every file processed"),
        )
        .arg(
            Arg::new(options::NO_PRESERVE_ROOT)
                .long(options::NO_PRESERVE_ROOT)
                .help("do not treat '/' specially (the default)"),
        )
        .arg(
            Arg::new(options::PRESERVE_ROOT)
                .long(options::PRESERVE_ROOT)
                .help("fail to operate recursively on '/'"),
        )
        .arg(
            Arg::new(options::RECURSIVE)
                .long(options::RECURSIVE)
                .short('R')
                .help("change files and directories recursively"),
        )
        .arg(
            Arg::new(options::REFERENCE)
                .long("reference")
                .takes_value(true)
                .help("use RFILE's mode instead of MODE values"),
        )
        .arg(
            Arg::new(options::MODE)
                .required_unless_present(options::REFERENCE)
                .takes_value(true),
            // It would be nice if clap could parse with delimiter, e.g. "g-x,u+x",
            // however .multiple_occurrences(true) cannot be used here because FILE already needs that.
            // Only one positional argument with .multiple_occurrences(true) set is allowed per command
        )
        .arg(
            Arg::new(options::FILE)
                .required_unless_present(options::MODE)
                .multiple_occurrences(true),
        )
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
    fn chmod(&self, files: &[String]) -> UResult<()> {
        let mut r = Ok(());

        for filename in files {
            let filename = &filename[..];
            let file = Path::new(filename);
            if !file.exists() {
                if is_symlink(file) {
                    println!(
                        "failed to change mode of {} from 0000 (---------) to 0000 (---------)",
                        filename.quote()
                    );
                    if !self.quiet {
                        return Err(USimpleError::new(
                            1,
                            format!("cannot operate on dangling symlink {}", filename.quote()),
                        ));
                    }
                } else if !self.quiet {
                    return Err(USimpleError::new(
                        1,
                        format!(
                            "cannot access {}: No such file or directory",
                            filename.quote()
                        ),
                    ));
                }
                return Err(ExitCode::new(1));
            }
            if self.recursive && self.preserve_root && filename == "/" {
                return Err(USimpleError::new(
                    1,
                    format!(
                        "it is dangerous to operate recursively on {}\nuse --no-preserve-root to override this failsafe",
                        filename.quote()
                    )
                ));
            }
            if !self.recursive {
                r = self.chmod_file(file).and(r);
            } else {
                r = self.walk_dir(file);
            }
        }
        r
    }

    fn walk_dir(&self, file_path: &Path) -> UResult<()> {
        let mut r = self.chmod_file(file_path);
        if !is_symlink(file_path) && file_path.is_dir() {
            for dir_entry in file_path.read_dir()? {
                let path = dir_entry?.path();
                if !is_symlink(&path) {
                    r = self.walk_dir(path.as_path());
                }
            }
        }
        r
    }

    #[cfg(windows)]
    fn chmod_file(&self, file: &Path) -> UResult<()> {
        // chmod is useless on Windows
        // it doesn't set any permissions at all
        // instead it just sets the readonly attribute on the file
        Ok(())
    }
    #[cfg(unix)]
    fn chmod_file(&self, file: &Path) -> UResult<()> {
        use uucore::mode::get_umask;

        let fperm = match fs::metadata(file) {
            Ok(meta) => meta.mode() & 0o7777,
            Err(err) => {
                if is_symlink(file) {
                    if self.verbose {
                        println!(
                            "neither symbolic link {} nor referent has been changed",
                            file.quote()
                        );
                    }
                    return Ok(());
                } else if err.kind() == std::io::ErrorKind::PermissionDenied {
                    // These two filenames would normally be conditionally
                    // quoted, but GNU's tests expect them to always be quoted
                    return Err(USimpleError::new(
                        1,
                        format!("{}: Permission denied", file.quote()),
                    ));
                } else {
                    return Err(USimpleError::new(1, format!("{}: {}", file.quote(), err)));
                }
            }
        };
        match self.fmode {
            Some(mode) => self.change_file(fperm, mode, file)?,
            None => {
                let cmode_unwrapped = self.cmode.clone().unwrap();
                let mut new_mode = fperm;
                let mut naively_expected_new_mode = new_mode;
                for mode in cmode_unwrapped.split(',') {
                    // cmode is guaranteed to be Some in this case
                    let arr: &[char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
                    let result = if mode.contains(arr) {
                        mode::parse_numeric(new_mode, mode, file.is_dir()).map(|v| (v, v))
                    } else {
                        mode::parse_symbolic(new_mode, mode, get_umask(), file.is_dir()).map(|m| {
                            // calculate the new mode as if umask was 0
                            let naive_mode = mode::parse_symbolic(
                                naively_expected_new_mode,
                                mode,
                                0,
                                file.is_dir(),
                            )
                            .unwrap(); // we know that mode must be valid, so this cannot fail
                            (m, naive_mode)
                        })
                    };
                    match result {
                        Ok((mode, naive_mode)) => {
                            new_mode = mode;
                            naively_expected_new_mode = naive_mode;
                        }
                        Err(f) => {
                            if !self.quiet {
                                return Err(USimpleError::new(1, f));
                            } else {
                                return Err(ExitCode::new(1));
                            }
                        }
                    }
                }
                self.change_file(fperm, new_mode, file)?;
                // if a permission would have been removed if umask was 0, but it wasn't because umask was not 0, print an error and fail
                if (new_mode & !naively_expected_new_mode) != 0 {
                    return Err(USimpleError::new(
                        1,
                        format!(
                            "{}: new permissions are {}, not {}",
                            file.maybe_quote(),
                            display_permissions_unix(new_mode as mode_t, false),
                            display_permissions_unix(naively_expected_new_mode as mode_t, false)
                        ),
                    ));
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
                    "mode of {} retained as {:04o} ({})",
                    file.quote(),
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
                println!(
                    "failed to change mode of file {} from {:04o} ({}) to {:04o} ({})",
                    file.quote(),
                    fperm,
                    display_permissions_unix(fperm as mode_t, false),
                    mode,
                    display_permissions_unix(mode as mode_t, false)
                );
            }
            Err(1)
        } else {
            if self.verbose || self.changes {
                println!(
                    "mode of {} changed from {:04o} ({}) to {:04o} ({})",
                    file.quote(),
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
