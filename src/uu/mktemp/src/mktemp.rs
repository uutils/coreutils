// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (paths) GPGHome findxs

use clap::{builder::ValueParser, crate_version, Arg, ArgAction, ArgMatches, Command};
use uucore::display::{println_verbatim, Quotable};
use uucore::error::{FromIo, UError, UResult, UUsageError};
use uucore::{format_usage, help_about, help_usage};

use std::env;
use std::ffi::OsStr;
use std::io::ErrorKind;
use std::iter;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::PermissionsExt;

use rand::Rng;
use tempfile::Builder;
use thiserror::Error;

const ABOUT: &str = help_about!("mktemp.md");
const USAGE: &str = help_usage!("mktemp.md");

static DEFAULT_TEMPLATE: &str = "tmp.XXXXXXXXXX";

static OPT_DIRECTORY: &str = "directory";
static OPT_DRY_RUN: &str = "dry-run";
static OPT_QUIET: &str = "quiet";
static OPT_SUFFIX: &str = "suffix";
static OPT_TMPDIR: &str = "tmpdir";
static OPT_P: &str = "p";
static OPT_T: &str = "t";

static ARG_TEMPLATE: &str = "template";

#[cfg(not(windows))]
const TMPDIR_ENV_VAR: &str = "TMPDIR";
#[cfg(windows)]
const TMPDIR_ENV_VAR: &str = "TMP";

#[derive(Debug, Error)]
enum MkTempError {
    #[error("could not persist file {path}", path = .0.quote())]
    PersistError(PathBuf),

    #[error("with --suffix, template {template} must end in X", template = .0.quote())]
    MustEndInX(String),

    #[error("too few X's in template {template}", template = .0.quote())]
    TooFewXs(String),

    /// The template prefix contains a path separator (e.g. `"a/bXXX"`).
    #[error("invalid template, {template}, contains directory separator", template = .0.quote())]
    PrefixContainsDirSeparator(String),

    /// The template suffix contains a path separator (e.g. `"XXXa/b"`).
    #[error("invalid suffix {suffix}, contains directory separator", suffix = .0.quote())]
    SuffixContainsDirSeparator(String),

    #[error("invalid template, {template}; with --tmpdir, it may not be absolute", template = .0.quote())]
    InvalidTemplate(String),

    #[error("too many templates")]
    TooManyTemplates,

    /// When a specified temporary directory could not be found.
    #[error("failed to create {template_type} via template {template}: No such file or directory",
            template_type = .0,
            template = .1.quote())]
    NotFound(String, String),
}

impl UError for MkTempError {
    fn usage(&self) -> bool {
        matches!(self, Self::TooManyTemplates)
    }
}

/// Options parsed from the command-line.
///
/// This provides a layer of indirection between the application logic
/// and the argument parsing library `clap`, allowing each to vary
/// independently.
#[derive(Clone)]
pub struct Options {
    /// Whether to create a temporary directory instead of a file.
    pub directory: bool,

    /// Whether to just print the name of a file that would have been created.
    pub dry_run: bool,

    /// Whether to suppress file creation error messages.
    pub quiet: bool,

    /// The directory in which to create the temporary file.
    ///
    /// If `None`, the file will be created in the current directory.
    pub tmpdir: Option<PathBuf>,

    /// The suffix to append to the temporary file, if any.
    pub suffix: Option<String>,

    /// Whether to treat the template argument as a single file path component.
    pub treat_as_template: bool,

    /// The template to use for the name of the temporary file.
    pub template: String,
}

impl Options {
    fn from(matches: &ArgMatches) -> Self {
        let tmpdir = matches
            .get_one::<PathBuf>(OPT_TMPDIR)
            .or_else(|| matches.get_one::<PathBuf>(OPT_P))
            .cloned();
        let (tmpdir, template) = match matches.get_one::<String>(ARG_TEMPLATE) {
            // If no template argument is given, `--tmpdir` is implied.
            None => {
                let tmpdir = Some(tmpdir.unwrap_or_else(env::temp_dir));
                let template = DEFAULT_TEMPLATE;
                (tmpdir, template.to_string())
            }
            Some(template) => {
                let tmpdir = if env::var(TMPDIR_ENV_VAR).is_ok() && matches.get_flag(OPT_T) {
                    env::var_os(TMPDIR_ENV_VAR).map(|t| t.into())
                } else if tmpdir.is_some() {
                    tmpdir
                } else if matches.get_flag(OPT_T) || matches.contains_id(OPT_TMPDIR) {
                    // If --tmpdir is given without an argument, or -t is given
                    // export in TMPDIR
                    Some(env::temp_dir())
                } else {
                    None
                };
                (tmpdir, template.to_string())
            }
        };
        Self {
            directory: matches.get_flag(OPT_DIRECTORY),
            dry_run: matches.get_flag(OPT_DRY_RUN),
            quiet: matches.get_flag(OPT_QUIET),
            tmpdir,
            suffix: matches.get_one::<String>(OPT_SUFFIX).map(String::from),
            treat_as_template: matches.get_flag(OPT_T),
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
    directory: PathBuf,

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
                    // If a suffix is specified, the error message includes the template without the suffix.
                    Some(_) => options
                        .template
                        .chars()
                        .take(options.template.len())
                        .collect::<String>(),
                    None => options.template,
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
        let prefix_from_option = tmpdir.clone().unwrap_or_default();
        let prefix_from_template = &options.template[..i];
        let prefix = Path::new(&prefix_from_option)
            .join(prefix_from_template)
            .display()
            .to_string();
        if options.treat_as_template && prefix_from_template.contains(MAIN_SEPARATOR) {
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
            (prefix, String::new())
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
        let suffix_from_option = options.suffix.unwrap_or_default();
        let suffix_from_template = &options.template[j..];
        let suffix = format!("{suffix_from_template}{suffix_from_option}");
        if suffix.contains(MAIN_SEPARATOR) {
            return Err(MkTempError::SuffixContainsDirSeparator(suffix));
        }

        // The number of random characters in the template.
        //
        // For example, if the template is "abcXXXXyz", then the number of
        // random characters is four.
        let num_rand_chars = j - i;

        Ok(Self {
            directory: directory.into(),
            prefix,
            num_rand_chars,
            suffix,
        })
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args: Vec<_> = args.collect();
    let matches = match uu_app().try_get_matches_from(&args) {
        Ok(m) => m,
        Err(e) => {
            if e.kind() == clap::error::ErrorKind::TooManyValues
                && e.context().any(|(kind, val)| {
                    kind == clap::error::ContextKind::InvalidArg
                        && val == &clap::error::ContextValue::String("[template]".into())
                })
            {
                return Err(UUsageError::new(1, "too many templates"));
            }
            return Err(e.into());
        }
    };

    // Parse command-line options into a format suitable for the
    // application logic.
    let options = Options::from(&matches);

    if env::var("POSIXLY_CORRECT").is_ok() {
        // If POSIXLY_CORRECT was set, template MUST be the last argument.
        if matches.contains_id(ARG_TEMPLATE) {
            // Template argument was provided, check if was the last one.
            if args.last().unwrap() != OsStr::new(&options.template) {
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

    let res = if suppress_file_err {
        // Mapping all UErrors to ExitCodes prevents the errors from being printed
        res.map_err(|e| e.code().into())
    } else {
        res
    };
    println_verbatim(res?).map_err_context(|| "failed to print directory name".to_owned())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_DIRECTORY)
                .short('d')
                .long(OPT_DIRECTORY)
                .help("Make a directory instead of a file")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_DRY_RUN)
                .short('u')
                .long(OPT_DRY_RUN)
                .help("do not create anything; merely print a name (unsafe)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_QUIET)
                .short('q')
                .long("quiet")
                .help("Fail silently if an error occurs.")
                .action(ArgAction::SetTrue),
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
            Arg::new(OPT_P)
                .short('p')
                .help("short form of --tmpdir")
                .value_name("DIR")
                .num_args(1)
                .value_parser(ValueParser::path_buf())
                .value_hint(clap::ValueHint::DirPath),
        )
        .arg(
            Arg::new(OPT_TMPDIR)
                .long(OPT_TMPDIR)
                .help(
                    "interpret TEMPLATE relative to DIR; if DIR is not specified, use \
                     $TMPDIR ($TMP on windows) if set, else /tmp. With this option, \
                     TEMPLATE must not be an absolute name; unlike with -t, TEMPLATE \
                     may contain slashes, but mktemp creates only the final component",
                )
                .value_name("DIR")
                // Allows use of default argument just by setting --tmpdir. Else,
                // use provided input to generate tmpdir
                .num_args(0..=1)
                // Require an equals to avoid ambiguity if no tmpdir is supplied
                .require_equals(true)
                .overrides_with(OPT_P)
                .value_parser(ValueParser::path_buf())
                .value_hint(clap::ValueHint::DirPath),
        )
        .arg(
            Arg::new(OPT_T)
                .short('t')
                .help(
                    "Generate a template (using the supplied prefix and TMPDIR \
                (TMP on windows) if set) to create a filename template [deprecated]",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new(ARG_TEMPLATE).num_args(..=1))
}

fn dry_exec(tmpdir: &Path, prefix: &str, rand: usize, suffix: &str) -> UResult<PathBuf> {
    let len = prefix.len() + suffix.len() + rand;
    let mut buf = Vec::with_capacity(len);
    buf.extend(prefix.as_bytes());
    // In Rust v1.82.0, use `repeat_n`:
    // <https://doc.rust-lang.org/std/iter/fn.repeat_n.html>
    buf.extend(iter::repeat(b'X').take(rand));
    buf.extend(suffix.as_bytes());

    // Randomize.
    let bytes = &mut buf[prefix.len()..prefix.len() + rand];
    rand::rng().fill(bytes);
    for byte in bytes {
        *byte = match *byte % 62 {
            v @ 0..=9 => v + b'0',
            v @ 10..=35 => v - 10 + b'a',
            v @ 36..=61 => v - 36 + b'A',
            _ => unreachable!(),
        }
    }
    // We guarantee utf8.
    let buf = String::from_utf8(buf).unwrap();
    let tmpdir = Path::new(tmpdir).join(buf);
    Ok(tmpdir)
}

/// Create a temporary directory with the given parameters.
///
/// This function creates a temporary directory as a subdirectory of
/// `dir`. The name of the directory is the concatenation of `prefix`,
/// a string of `rand` random characters, and `suffix`. The
/// permissions of the directory are set to `u+rwx`
///
/// # Errors
///
/// If the temporary directory could not be written to disk or if the
/// given directory `dir` does not exist.
fn make_temp_dir(dir: &Path, prefix: &str, rand: usize, suffix: &str) -> UResult<PathBuf> {
    let mut builder = Builder::new();
    builder.prefix(prefix).rand_bytes(rand).suffix(suffix);
    match builder.tempdir_in(dir) {
        Ok(d) => {
            // `into_path` consumes the TempDir without removing it
            let path = d.into_path();
            #[cfg(not(windows))]
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
            Ok(path)
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let filename = format!("{}{}{}", prefix, "X".repeat(rand), suffix);
            let path = Path::new(dir).join(filename);
            let s = path.display().to_string();
            Err(MkTempError::NotFound("directory".to_string(), s).into())
        }
        Err(e) => Err(e.into()),
    }
}

/// Create a temporary file with the given parameters.
///
/// This function creates a temporary file in the directory `dir`. The
/// name of the file is the concatenation of `prefix`, a string of
/// `rand` random characters, and `suffix`. The permissions of the
/// file are set to `u+rw`.
///
/// # Errors
///
/// If the file could not be written to disk or if the directory does
/// not exist.
fn make_temp_file(dir: &Path, prefix: &str, rand: usize, suffix: &str) -> UResult<PathBuf> {
    let mut builder = Builder::new();
    builder.prefix(prefix).rand_bytes(rand).suffix(suffix);
    match builder.tempfile_in(dir) {
        // `keep` ensures that the file is not deleted
        Ok(named_tempfile) => match named_tempfile.keep() {
            Ok((_, pathbuf)) => Ok(pathbuf),
            Err(e) => Err(MkTempError::PersistError(e.file.path().to_path_buf()).into()),
        },
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let filename = format!("{}{}{}", prefix, "X".repeat(rand), suffix);
            let path = Path::new(dir).join(filename);
            let s = path.display().to_string();
            Err(MkTempError::NotFound("file".to_string(), s).into())
        }
        Err(e) => Err(e.into()),
    }
}

fn exec(dir: &Path, prefix: &str, rand: usize, suffix: &str, make_dir: bool) -> UResult<PathBuf> {
    let path = if make_dir {
        make_temp_dir(dir, prefix, rand, suffix)?
    } else {
        make_temp_file(dir, prefix, rand, suffix)?
    };

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

    Ok(path)
}

/// Create a temporary file or directory
///
/// Behavior is determined by the `options` parameter, see [`Options`] for details.
pub fn mktemp(options: &Options) -> UResult<PathBuf> {
    // Parse file path parameters from the command-line options.
    let Params {
        directory: tmpdir,
        prefix,
        num_rand_chars: rand,
        suffix,
    } = Params::from(options.clone())?;

    // Create the temporary file or directory, or simulate creating it.
    if options.dry_run {
        dry_exec(&tmpdir, &prefix, rand, &suffix)
    } else {
        exec(&tmpdir, &prefix, rand, &suffix, options.directory)
    }
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
