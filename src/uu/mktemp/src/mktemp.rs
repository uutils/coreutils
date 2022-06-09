// This file is part of the uutils coreutils package.
//
// (c) Sunrin SHIMURA
// Collaborator: Jian Zeng
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (paths) GPGHome findxs

use clap::{crate_version, Arg, ArgMatches, Command};
use uucore::display::{println_verbatim, Quotable};
use uucore::error::{FromIo, UError, UResult};
use uucore::format_usage;

use std::env;
use std::error::Error;
use std::fmt::Display;
use std::iter;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::PermissionsExt;

use rand::Rng;
use tempfile::Builder;

static ABOUT: &str = "create a temporary file or directory.";
const USAGE: &str = "{} [OPTION]... [TEMPLATE]";

static DEFAULT_TEMPLATE: &str = "tmp.XXXXXXXXXX";

static OPT_DIRECTORY: &str = "directory";
static OPT_DRY_RUN: &str = "dry-run";
static OPT_QUIET: &str = "quiet";
static OPT_SUFFIX: &str = "suffix";
static OPT_TMPDIR: &str = "tmpdir";
static OPT_T: &str = "t";

static ARG_TEMPLATE: &str = "template";

#[derive(Debug)]
enum MkTempError {
    PersistError(PathBuf),
    MustEndInX(String),
    TooFewXs(String),

    /// The template prefix contains a path separator (e.g. `"a/bXXX"`).
    PrefixContainsDirSeparator(String),

    /// The template suffix contains a path separator (e.g. `"XXXa/b"`).
    SuffixContainsDirSeparator(String),
    InvalidTemplate(String),
    TooManyTemplates,
}

impl UError for MkTempError {
    fn usage(&self) -> bool {
        matches!(self, Self::TooManyTemplates)
    }
}

impl Error for MkTempError {}

impl Display for MkTempError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use MkTempError::*;
        match self {
            PersistError(p) => write!(f, "could not persist file {}", p.quote()),
            MustEndInX(s) => write!(f, "with --suffix, template {} must end in X", s.quote()),
            TooFewXs(s) => write!(f, "too few X's in template {}", s.quote()),
            PrefixContainsDirSeparator(s) => {
                write!(
                    f,
                    "invalid template, {}, contains directory separator",
                    s.quote()
                )
            }
            SuffixContainsDirSeparator(s) => {
                write!(
                    f,
                    "invalid suffix {}, contains directory separator",
                    s.quote()
                )
            }
            InvalidTemplate(s) => write!(
                f,
                "invalid template, {}; with --tmpdir, it may not be absolute",
                s.quote()
            ),
            TooManyTemplates => {
                write!(f, "too many templates")
            }
        }
    }
}

/// Options parsed from the command-line.
///
/// This provides a layer of indirection between the application logic
/// and the argument parsing library `clap`, allowing each to vary
/// independently.
struct Options {
    /// Whether to create a temporary directory instead of a file.
    directory: bool,

    /// Whether to just print the name of a file that would have been created.
    dry_run: bool,

    /// Whether to suppress file creation error messages.
    quiet: bool,

    /// The directory in which to create the temporary file.
    ///
    /// If `None`, the file will be created in the current directory.
    tmpdir: Option<String>,

    /// The suffix to append to the temporary file, if any.
    suffix: Option<String>,

    /// Whether to treat the template argument as a single file path component.
    treat_as_template: bool,

    /// The template to use for the name of the temporary file.
    template: String,
}

/// Decide whether the argument to `--tmpdir` should actually be the template.
///
/// This function is required to work around a limitation of `clap`,
/// the command-line argument parsing library. In case the command
/// line is
///
/// ```sh
/// mktemp --tmpdir XXX
/// ```
///
/// the program should behave like
///
/// ```sh
/// mktemp --tmpdir=${TMPDIR:-/tmp} XXX
/// ```
///
/// However, `clap` thinks that `XXX` is the value of the `--tmpdir`
/// option. This function returns `true` in this case and `false`
/// in all other cases.
fn is_tmpdir_argument_actually_the_template(matches: &ArgMatches) -> bool {
    if !matches.is_present(ARG_TEMPLATE) {
        if let Some(tmpdir) = matches.value_of(OPT_TMPDIR) {
            if !Path::new(tmpdir).is_dir() && tmpdir.contains("XXX") {
                return true;
            }
        }
    }
    false
}

impl Options {
    fn from(matches: &ArgMatches) -> Self {
        // Special case to work around a limitation of `clap`; see
        // `is_tmpdir_argument_actually_the_template()` for more
        // information.
        //
        // Fixed in clap 3
        // See https://github.com/clap-rs/clap/pull/1587
        let (tmpdir, template) = if is_tmpdir_argument_actually_the_template(matches) {
            let tmpdir = Some(env::temp_dir().display().to_string());
            let template = matches.value_of(OPT_TMPDIR).unwrap().to_string();
            (tmpdir, template)
        } else {
            let tmpdir = matches.value_of(OPT_TMPDIR).map(String::from);
            let template = matches
                .value_of(ARG_TEMPLATE)
                .unwrap_or(DEFAULT_TEMPLATE)
                .to_string();
            (tmpdir, template)
        };
        Self {
            directory: matches.is_present(OPT_DIRECTORY),
            dry_run: matches.is_present(OPT_DRY_RUN),
            quiet: matches.is_present(OPT_QUIET),
            tmpdir,
            suffix: matches.value_of(OPT_SUFFIX).map(String::from),
            treat_as_template: matches.is_present(OPT_T),
            template,
        }
    }
}

/// Parameters that control the path to and name of the temporary file.
///
/// The temporary file will be created at
///
/// ```text
/// {directory}/{prefix}{XXX}{suffix}
/// ```
///
/// where `{XXX}` is a sequence of random characters whose length is
/// `num_rand_chars`.
struct Params {
    /// The directory that will contain the temporary file.
    directory: String,

    /// The (non-random) prefix of the temporary file.
    prefix: String,

    /// The number of random characters in the name of the temporary file.
    num_rand_chars: usize,

    /// The (non-random) suffix of the temporary file.
    suffix: String,
}

/// Find the start and end indices of the last contiguous block of Xs.
///
/// If no contiguous block of at least three Xs could be found, this
/// function returns `None`.
///
/// # Examples
///
/// ```rust,ignore
/// assert_eq!(find_last_contiguous_block_of_xs("XXX_XXX"), Some((4, 7)));
/// assert_eq!(find_last_contiguous_block_of_xs("aXbXcX"), None);
/// ```
fn find_last_contiguous_block_of_xs(s: &str) -> Option<(usize, usize)> {
    let j = s.rfind("XXX")? + 3;
    let i = s[..j].rfind(|c| c != 'X').map_or(0, |i| i + 1);
    Some((i, j))
}

impl Params {
    fn from(options: Options) -> Result<Self, MkTempError> {
        // The template argument must end in 'X' if a suffix option is given.
        if options.suffix.is_some() && !options.template.ends_with('X') {
            return Err(MkTempError::MustEndInX(options.template));
        }

        // Get the start and end indices of the randomized part of the template.
        //
        // For example, if the template is "abcXXXXyz", then `i` is 3 and `j` is 7.
        let (i, j) = match find_last_contiguous_block_of_xs(&options.template) {
            None => {
                let s = match options.suffix {
                    None => options.template,
                    Some(s) => format!("{}{}", options.template, s),
                };
                return Err(MkTempError::TooFewXs(s));
            }
            Some(indices) => indices,
        };

        // Combine the directory given as an option and the prefix of the template.
        //
        // For example, if `tmpdir` is "a/b" and the template is "c/dXXX",
        // then `prefix` is "a/b/c/d".
        let tmpdir = options.tmpdir;
        let prefix_from_option = tmpdir.clone().unwrap_or_else(|| "".to_string());
        let prefix_from_template = &options.template[..i];
        let prefix = Path::new(&prefix_from_option)
            .join(prefix_from_template)
            .display()
            .to_string();
        if options.treat_as_template && prefix.contains(MAIN_SEPARATOR) {
            return Err(MkTempError::PrefixContainsDirSeparator(options.template));
        }
        if tmpdir.is_some() && Path::new(prefix_from_template).is_absolute() {
            return Err(MkTempError::InvalidTemplate(options.template));
        }

        // Split the parent directory from the file part of the prefix.
        //
        // For example, if `prefix` is "a/b/c/d", then `directory` is
        // "a/b/c" is `prefix` gets reassigned to "d".
        let (directory, prefix) = if prefix.ends_with(MAIN_SEPARATOR) {
            (prefix, "".to_string())
        } else {
            let path = Path::new(&prefix);
            let directory = match path.parent() {
                None => String::new(),
                Some(d) => d.display().to_string(),
            };
            let prefix = match path.file_name() {
                None => String::new(),
                Some(f) => f.to_str().unwrap().to_string(),
            };
            (directory, prefix)
        };

        // Combine the suffix from the template with the suffix given as an option.
        //
        // For example, if the suffix command-line argument is ".txt" and
        // the template is "XXXabc", then `suffix` is "abc.txt".
        let suffix_from_option = options.suffix.unwrap_or_else(|| "".to_string());
        let suffix_from_template = &options.template[j..];
        let suffix = format!("{}{}", suffix_from_template, suffix_from_option);
        if suffix.contains(MAIN_SEPARATOR) {
            return Err(MkTempError::SuffixContainsDirSeparator(suffix));
        }

        // The number of random characters in the template.
        //
        // For example, if the template is "abcXXXXyz", then the number of
        // random characters is four.
        let num_rand_chars = j - i;

        Ok(Self {
            directory,
            prefix,
            num_rand_chars,
            suffix,
        })
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_str_lossy().accept_any();

    let matches = uu_app().try_get_matches_from(&args)?;

    // Parse command-line options into a format suitable for the
    // application logic.
    let options = Options::from(&matches);

    if env::var("POSIXLY_CORRECT").is_ok() {
        // If POSIXLY_CORRECT was set, template MUST be the last argument.
        if is_tmpdir_argument_actually_the_template(&matches) || matches.is_present(ARG_TEMPLATE) {
            // Template argument was provided, check if was the last one.
            if args.last().unwrap() != &options.template {
                return Err(Box::new(MkTempError::TooManyTemplates));
            }
        }
    }

    let dry_run = options.dry_run;
    let suppress_file_err = options.quiet;
    let make_dir = options.directory;

    // Parse file path parameters from the command-line options.
    let Params {
        directory: tmpdir,
        prefix,
        num_rand_chars: rand,
        suffix,
    } = Params::from(options)?;

    // Create the temporary file or directory, or simulate creating it.
    let res = if dry_run {
        dry_exec(&tmpdir, &prefix, rand, &suffix)
    } else {
        exec(&tmpdir, &prefix, rand, &suffix, make_dir)
    };

    if suppress_file_err {
        // Mapping all UErrors to ExitCodes prevents the errors from being printed
        res.map_err(|e| e.code().into())
    } else {
        res
    }
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_DIRECTORY)
                .short('d')
                .long(OPT_DIRECTORY)
                .help("Make a directory instead of a file"),
        )
        .arg(
            Arg::new(OPT_DRY_RUN)
                .short('u')
                .long(OPT_DRY_RUN)
                .help("do not create anything; merely print a name (unsafe)"),
        )
        .arg(
            Arg::new(OPT_QUIET)
                .short('q')
                .long("quiet")
                .help("Fail silently if an error occurs."),
        )
        .arg(
            Arg::new(OPT_SUFFIX)
                .long(OPT_SUFFIX)
                .help(
                    "append SUFFIX to TEMPLATE; SUFFIX must not contain a path separator. \
                     This option is implied if TEMPLATE does not end with X.",
                )
                .value_name("SUFFIX"),
        )
        .arg(
            Arg::new(OPT_TMPDIR)
                .short('p')
                .long(OPT_TMPDIR)
                .help(
                    "interpret TEMPLATE relative to DIR; if DIR is not specified, use \
                     $TMPDIR ($TMP on windows) if set, else /tmp. With this option, TEMPLATE must not \
                     be an absolute name; unlike with -t, TEMPLATE may contain \
                     slashes, but mktemp creates only the final component",
                )
                .value_name("DIR")
                .value_hint(clap::ValueHint::DirPath),
        )
        .arg(Arg::new(OPT_T).short('t').help(
            "Generate a template (using the supplied prefix and TMPDIR (TMP on windows) if set) \
             to create a filename template [deprecated]",
        ))
        .arg(
            Arg::new(ARG_TEMPLATE)
                .multiple_occurrences(false)
                .takes_value(true)
                .max_values(1)
        )
}

pub fn dry_exec(tmpdir: &str, prefix: &str, rand: usize, suffix: &str) -> UResult<()> {
    let len = prefix.len() + suffix.len() + rand;
    let mut buf = Vec::with_capacity(len);
    buf.extend(prefix.as_bytes());
    buf.extend(iter::repeat(b'X').take(rand));
    buf.extend(suffix.as_bytes());

    // Randomize.
    let bytes = &mut buf[prefix.len()..prefix.len() + rand];
    rand::thread_rng().fill(bytes);
    for byte in bytes.iter_mut() {
        *byte = match *byte % 62 {
            v @ 0..=9 => (v + b'0'),
            v @ 10..=35 => (v - 10 + b'a'),
            v @ 36..=61 => (v - 36 + b'A'),
            _ => unreachable!(),
        }
    }
    // We guarantee utf8.
    let buf = String::from_utf8(buf).unwrap();
    let tmpdir = Path::new(tmpdir).join(buf);
    println_verbatim(tmpdir).map_err_context(|| "failed to print directory name".to_owned())
}

fn exec(dir: &str, prefix: &str, rand: usize, suffix: &str, make_dir: bool) -> UResult<()> {
    let context = || {
        format!(
            "failed to create file via template '{}{}{}'",
            prefix,
            "X".repeat(rand),
            suffix
        )
    };

    let mut builder = Builder::new();
    builder.prefix(prefix).rand_bytes(rand).suffix(suffix);

    let path = if make_dir {
        builder
            .tempdir_in(&dir)
            .map_err_context(context)?
            .into_path() // `into_path` consumes the TempDir without removing it
    } else {
        builder
            .tempfile_in(&dir)
            .map_err_context(context)?
            .keep() // `keep` ensures that the file is not deleted
            .map_err(|e| MkTempError::PersistError(e.file.path().to_path_buf()))?
            .1
    };

    #[cfg(not(windows))]
    if make_dir {
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
    }

    // Get just the last component of the path to the created
    // temporary file or directory.
    let filename = path.file_name();
    let filename = filename.unwrap().to_str().unwrap();

    // Join the directory to the path to get the path to print. We
    // cannot use the path returned by the `Builder` because it gives
    // the absolute path and we need to return a filename that matches
    // the template given on the command-line which might be a
    // relative path.
    let path = Path::new(dir).join(filename);

    println_verbatim(path).map_err_context(|| "failed to print directory name".to_owned())
}

#[cfg(test)]
mod tests {
    use crate::find_last_contiguous_block_of_xs as findxs;

    #[test]
    fn test_find_last_contiguous_block_of_xs() {
        assert_eq!(findxs("XXX"), Some((0, 3)));
        assert_eq!(findxs("XXX_XXX"), Some((4, 7)));
        assert_eq!(findxs("XXX_XXX_XXX"), Some((8, 11)));
        assert_eq!(findxs("aaXXXbb"), Some((2, 5)));
        assert_eq!(findxs(""), None);
        assert_eq!(findxs("X"), None);
        assert_eq!(findxs("XX"), None);
        assert_eq!(findxs("aXbXcX"), None);
        assert_eq!(findxs("aXXbXXcXX"), None);
    }
}
