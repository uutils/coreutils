// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::ArgAction;
use clap::{Arg, Command};
use std::env;
use std::io;
use std::path::PathBuf;
use uucore::format_usage;

use uucore::display::println_verbatim;
use uucore::error::{FromIo, UResult};

use uucore::translate;
const OPT_LOGICAL: &str = "logical";
const OPT_PHYSICAL: &str = "physical";

fn physical_path() -> io::Result<PathBuf> {
    // std::env::current_dir() is a thin wrapper around libc::getcwd().
    let path = env::current_dir()?;

    // On Unix, getcwd() must return the physical path:
    // https://pubs.opengroup.org/onlinepubs/9699919799/functions/getcwd.html
    #[cfg(unix)]
    {
        Ok(path)
    }

    // On Windows we have to resolve it.
    // On other systems we also resolve it, just in case.
    #[cfg(not(unix))]
    {
        path.canonicalize()
    }
}

fn logical_path() -> io::Result<PathBuf> {
    // getcwd() on Windows seems to include symlinks, so this is easy.
    #[cfg(windows)]
    {
        env::current_dir()
    }

    // If we're not on Windows we do things Unix-style.
    //
    // Typical Unix-like kernels don't actually keep track of the logical working
    // directory. They know the precise directory a process is in, and the getcwd()
    // syscall reconstructs a path from that.
    //
    // The logical working directory is maintained by the shell, in the $PWD
    // environment variable. So we check carefully if that variable looks
    // reasonable, and if not then we fall back to the physical path.
    //
    // POSIX: https://pubs.opengroup.org/onlinepubs/9699919799/utilities/pwd.html
    #[cfg(not(windows))]
    {
        use std::path::Path;
        fn looks_reasonable(path: &Path) -> bool {
            // First, check if it's an absolute path.
            if !path.has_root() {
                return false;
            }

            // Then, make sure there are no . or .. components.
            // Path::components() isn't useful here, it normalizes those out.

            // to_string_lossy() may allocate, but that's fine, we call this
            // only once per run. It may also lose information, but not any
            // information that we need for this check.
            if path
                .to_string_lossy()
                .split(std::path::is_separator)
                .any(|piece| piece == "." || piece == "..")
            {
                return false;
            }

            // Finally, check if it matches the directory we're in.
            #[cfg(unix)]
            {
                use std::fs::metadata;
                use std::os::unix::fs::MetadataExt;
                match (metadata(path), metadata(".")) {
                    (Ok(info1), Ok(info2)) => {
                        info1.dev() == info2.dev() && info1.ino() == info2.ino()
                    }
                    _ => false,
                }
            }

            #[cfg(not(unix))]
            {
                use std::fs::canonicalize;
                match (canonicalize(path), canonicalize(".")) {
                    (Ok(path1), Ok(path2)) => path1 == path2,
                    _ => false,
                }
            }
        }

        match env::var_os("PWD").map(PathBuf::from) {
            Some(value) if looks_reasonable(&value) => Ok(value),
            _ => env::current_dir(),
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;
    // if POSIXLY_CORRECT is set, we want to a logical resolution.
    // This produces a different output when doing mkdir -p a/b && ln -s a/b c && cd c && pwd
    // We should get c in this case instead of a/b at the end of the path
    let cwd = if matches.get_flag(OPT_PHYSICAL) {
        physical_path()
    } else if matches.get_flag(OPT_LOGICAL) || env::var("POSIXLY_CORRECT").is_ok() {
        logical_path()
    } else {
        physical_path()
    }
    .map_err_context(|| translate!("pwd-error-failed-to-get-current-directory"))?;

    // \\?\ is a prefix Windows gives to paths under certain circumstances,
    // including when canonicalizing them.
    // With the right extension trait we can remove it non-lossily, but
    // we print it lossily anyway, so no reason to bother.
    #[cfg(windows)]
    let cwd = cwd
        .to_string_lossy()
        .strip_prefix(r"\\?\")
        .map(Into::into)
        .unwrap_or(cwd);

    println_verbatim(cwd)
        .map_err_context(|| translate!("pwd-error-failed-to-print-current-directory"))?;
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("pwd-about"))
        .override_usage(format_usage(&translate!("pwd-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_LOGICAL)
                .short('L')
                .long(OPT_LOGICAL)
                .help(translate!("pwd-help-logical"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_PHYSICAL)
                .short('P')
                .long(OPT_PHYSICAL)
                .overrides_with(OPT_LOGICAL)
                .help(translate!("pwd-help-physical"))
                .action(ArgAction::SetTrue),
        )
}
