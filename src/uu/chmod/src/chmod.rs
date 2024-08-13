// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) Chmoder cmode fmode fperm fref ugoa RFILE RFILE's

use clap::{crate_version, Arg, ArgAction, Command};
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{set_exit_code, ExitCode, UResult, USimpleError, UUsageError};
use uucore::fs::display_permissions_unix;
use uucore::libc::mode_t;
#[cfg(not(windows))]
use uucore::mode;
use uucore::{format_usage, help_about, help_section, help_usage, show, show_error};

const ABOUT: &str = help_about!("chmod.md");
const USAGE: &str = help_usage!("chmod.md");
const LONG_USAGE: &str = help_section!("after help", "chmod.md");

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

/// Extract negative modes (starting with '-') from the rest of the arguments.
///
/// This is mainly required for GNU compatibility, where "non-positional negative" modes are used
/// as the actual positional MODE. Some examples of these cases are:
/// * "chmod -w -r file", which is the same as "chmod -w,-r file"
/// * "chmod -w file -r", which is the same as "chmod -w,-r file"
///
/// These can currently not be handled by clap.
/// Therefore it might be possible that a pseudo MODE is inserted to pass clap parsing.
/// The pseudo MODE is later replaced by the extracted (and joined) negative modes.
fn extract_negative_modes(mut args: impl uucore::Args) -> (Option<String>, Vec<OsString>) {
    // we look up the args until "--" is found
    // "-mode" will be extracted into parsed_cmode_vec
    let (parsed_cmode_vec, pre_double_hyphen_args): (Vec<OsString>, Vec<OsString>) =
        args.by_ref().take_while(|a| a != "--").partition(|arg| {
            let arg = if let Some(arg) = arg.to_str() {
                arg.to_string()
            } else {
                return false;
            };
            arg.len() >= 2
                && arg.starts_with('-')
                && matches!(
                    arg.chars().nth(1).unwrap(),
                    'r' | 'w' | 'x' | 'X' | 's' | 't' | 'u' | 'g' | 'o' | '0'..='7'
                )
        });

    let mut clean_args = Vec::new();
    if !parsed_cmode_vec.is_empty() {
        // we need a pseudo cmode for clap, which won't be used later.
        // this is required because clap needs the default "chmod MODE FILE" scheme.
        clean_args.push("w".into());
    }
    clean_args.extend(pre_double_hyphen_args);

    if let Some(arg) = args.next() {
        // as there is still something left in the iterator, we previously consumed the "--"
        // -> add it to the args again
        clean_args.push("--".into());
        clean_args.push(arg);
    }
    clean_args.extend(args);

    let parsed_cmode = Some(
        parsed_cmode_vec
            .iter()
            .map(|s| s.to_str().unwrap())
            .collect::<Vec<&str>>()
            .join(","),
    )
    .filter(|s| !s.is_empty());
    (parsed_cmode, clean_args)
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let (parsed_cmode, args) = extract_negative_modes(args.skip(1)); // skip binary name
    let matches = uu_app().after_help(LONG_USAGE).try_get_matches_from(args)?;

    let changes = matches.get_flag(options::CHANGES);
    let quiet = matches.get_flag(options::QUIET);
    let verbose = matches.get_flag(options::VERBOSE);
    let preserve_root = matches.get_flag(options::PRESERVE_ROOT);
    let recursive = matches.get_flag(options::RECURSIVE);
    let fmode = match matches.get_one::<String>(options::REFERENCE) {
        Some(fref) => match fs::metadata(fref) {
            Ok(meta) => Some(meta.mode() & 0o7777),
            Err(err) => {
                return Err(USimpleError::new(
                    1,
                    format!("cannot stat attributes of {}: {}", fref.quote(), err),
                ))
            }
        },
        None => None,
    };

    let modes = matches.get_one::<String>(options::MODE);
    let cmode = if let Some(parsed_cmode) = parsed_cmode {
        parsed_cmode
    } else {
        modes.unwrap().to_string() // modes is required
    };
    // FIXME: enable non-utf8 paths
    let mut files: Vec<String> = matches
        .get_many::<String>(options::FILE)
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

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .args_override_self(true)
        .infer_long_args(true)
        .no_binary_name(true)
        .arg(
            Arg::new(options::CHANGES)
                .long(options::CHANGES)
                .short('c')
                .help("like verbose but report only when a change is made")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::QUIET)
                .long(options::QUIET)
                .visible_alias("silent")
                .short('f')
                .help("suppress most error messages")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .long(options::VERBOSE)
                .short('v')
                .help("output a diagnostic for every file processed")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_PRESERVE_ROOT)
                .long(options::NO_PRESERVE_ROOT)
                .help("do not treat '/' specially (the default)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PRESERVE_ROOT)
                .long(options::PRESERVE_ROOT)
                .help("fail to operate recursively on '/'")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RECURSIVE)
                .long(options::RECURSIVE)
                .short('R')
                .help("change files and directories recursively")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REFERENCE)
                .long("reference")
                .value_hint(clap::ValueHint::FilePath)
                .help("use RFILE's mode instead of MODE values"),
        )
        .arg(
            Arg::new(options::MODE).required_unless_present(options::REFERENCE), // It would be nice if clap could parse with delimiter, e.g. "g-x,u+x",
                                                                                 // however .multiple_occurrences(true) cannot be used here because FILE already needs that.
                                                                                 // Only one positional argument with .multiple_occurrences(true) set is allowed per command
        )
        .arg(
            Arg::new(options::FILE)
                .required_unless_present(options::MODE)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath),
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
                if file.is_symlink() {
                    if !self.quiet {
                        show!(USimpleError::new(
                            1,
                            format!("cannot operate on dangling symlink {}", filename.quote()),
                        ));
                    }
                    if self.verbose {
                        println!(
                            "failed to change mode of {} from 0000 (---------) to 1500 (r-x-----T)",
                            filename.quote()
                        );
                    }
                } else if !self.quiet {
                    show!(USimpleError::new(
                        1,
                        format!(
                            "cannot access {}: No such file or directory",
                            filename.quote()
                        )
                    ));
                }
                // GNU exits with exit code 1 even if -q or --quiet are passed
                // So we set the exit code, because it hasn't been set yet if `self.quiet` is true.
                set_exit_code(1);
                continue;
            }
            if self.recursive && self.preserve_root && filename == "/" {
                return Err(USimpleError::new(
                    1,
                    format!(
                        "it is dangerous to operate recursively on {}\nchmod: use --no-preserve-root to override this failsafe",
                        filename.quote()
                    )
                ));
            }
            if self.recursive {
                r = self.walk_dir(file);
            } else {
                r = self.chmod_file(file).and(r);
            }
        }
        r
    }

    fn walk_dir(&self, file_path: &Path) -> UResult<()> {
        let mut r = self.chmod_file(file_path);
        if !file_path.is_symlink() && file_path.is_dir() {
            for dir_entry in file_path.read_dir()? {
                let path = dir_entry?.path();
                if !path.is_symlink() {
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
                if file.is_symlink() {
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
                    let result = if mode.chars().any(|c| c.is_ascii_digit()) {
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
                            return if self.quiet {
                                Err(ExitCode::new(1))
                            } else {
                                Err(USimpleError::new(1, f))
                            };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_negative_modes() {
        // "chmod -w -r file" becomes "chmod -w,-r file". clap does not accept "-w,-r" as MODE.
        // Therefore, "w" is added as pseudo mode to pass clap.
        let (c, a) = extract_negative_modes(["-w", "-r", "file"].iter().map(OsString::from));
        assert_eq!(c, Some("-w,-r".to_string()));
        assert_eq!(a, ["w", "file"]);

        // "chmod -w file -r" becomes "chmod -w,-r file". clap does not accept "-w,-r" as MODE.
        // Therefore, "w" is added as pseudo mode to pass clap.
        let (c, a) = extract_negative_modes(["-w", "file", "-r"].iter().map(OsString::from));
        assert_eq!(c, Some("-w,-r".to_string()));
        assert_eq!(a, ["w", "file"]);

        // "chmod -w -- -r file" becomes "chmod -w -r file", where "-r" is interpreted as file.
        // Again, "w" is needed as pseudo mode.
        let (c, a) = extract_negative_modes(["-w", "--", "-r", "f"].iter().map(OsString::from));
        assert_eq!(c, Some("-w".to_string()));
        assert_eq!(a, ["w", "--", "-r", "f"]);

        // "chmod -- -r file" becomes "chmod -r file".
        let (c, a) = extract_negative_modes(["--", "-r", "file"].iter().map(OsString::from));
        assert_eq!(c, None);
        assert_eq!(a, ["--", "-r", "file"]);
    }
}
