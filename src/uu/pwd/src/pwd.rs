//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Derek Chiang <derekchiang93@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

use clap::{crate_version, Arg, Command};
use std::env;
use std::io;
use std::path::PathBuf;
use uucore::format_usage;

use uucore::display::println_verbatim;
use uucore::error::{FromIo, UResult};

static ABOUT: &str = "Display the full filename of the current working directory.";
const USAGE: &str = "{} [OPTION]... FILE...";
static OPT_LOGICAL: &str = "logical";
static OPT_PHYSICAL: &str = "physical";

fn physical_path() -> io::Result<PathBuf> {
    // std::env::current_dir() is a thin wrapper around libc::getcwd().

    // On Unix, getcwd() must return the physical path:
    // https://pubs.opengroup.org/onlinepubs/9699919799/functions/getcwd.html
    #[cfg(unix)]
    {
        env::current_dir()
    }

    // On Windows we have to resolve it.
    // On other systems we also resolve it, just in case.
    #[cfg(not(unix))]
    {
        env::current_dir().and_then(|path| path.canonicalize())
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
                let path_info = match metadata(path) {
                    Ok(info) => info,
                    Err(_) => return false,
                };
                let real_info = match metadata(".") {
                    Ok(info) => info,
                    Err(_) => return false,
                };
                if path_info.dev() != real_info.dev() || path_info.ino() != real_info.ino() {
                    return false;
                }
            }

            #[cfg(not(unix))]
            {
                use std::fs::canonicalize;
                let canon_path = match canonicalize(path) {
                    Ok(path) => path,
                    Err(_) => return false,
                };
                let real_path = match canonicalize(".") {
                    Ok(path) => path,
                    Err(_) => return false,
                };
                if canon_path != real_path {
                    return false;
                }
            }

            true
        }

        match env::var_os("PWD").map(PathBuf::from) {
            Some(value) if looks_reasonable(&value) => Ok(value),
            _ => env::current_dir(),
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);
    let cwd = if matches.is_present(OPT_LOGICAL) {
        logical_path()
    } else {
        physical_path()
    }
    .map_err_context(|| "failed to get current directory".to_owned())?;

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

    println_verbatim(&cwd).map_err_context(|| "failed to print current directory".to_owned())?;

    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_LOGICAL)
                .short('L')
                .long(OPT_LOGICAL)
                .help("use PWD from environment, even if it contains symlinks"),
        )
        .arg(
            Arg::new(OPT_PHYSICAL)
                .short('P')
                .long(OPT_PHYSICAL)
                .overrides_with(OPT_LOGICAL)
                .help("avoid all symlinks"),
        )
}
