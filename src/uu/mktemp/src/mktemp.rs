// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (paths) GPGHome findxs

use clap::builder::{TypedValueParser, ValueParserFactory};
use clap::{Arg, ArgAction, ArgMatches, Command};
use uucore::display::{Quotable, println_verbatim};
use uucore::error::{FromIo, UError, UResult, UUsageError};
use uucore::format_usage;
use uucore::translate;

use std::env;
use std::ffi::{OsStr, OsString};
use std::io::ErrorKind;
use std::iter;
use std::path::{MAIN_SEPARATOR, Path, PathBuf};

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::PermissionsExt;

use rand::Rng;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use tempfile::Builder;
use thiserror::Error;

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

const FALLBACK_TMPDIR: &str = "/tmp";

#[derive(Error, Debug)]
enum MkTempError {
    #[error("{}", translate!("mktemp-error-persist-file", "path" => .0.quote()))]
    PersistError(PathBuf),

    #[error("{}", translate!("mktemp-error-must-end-in-x", "template" => .0.quote()))]
    MustEndInX(String),

    #[error("{}", translate!("mktemp-error-too-few-xs", "template" => .0.quote()))]
    TooFewXs(String),

    #[error("{}", translate!("mktemp-error-prefix-contains-separator", "template" => .0.quote()))]
    PrefixContainsDirSeparator(String),

    #[error("{}", translate!("mktemp-error-suffix-contains-separator", "suffix" => .0.quote()))]
    SuffixContainsDirSeparator(String),

    #[error("{}", translate!("mktemp-error-invalid-template", "template" => .0.quote()))]
    InvalidTemplate(OsString),

    #[error("{}", translate!("mktemp-error-too-many-templates"))]
    TooManyTemplates,

    #[error("{}", translate!("mktemp-error-not-found", "template_type" => .0.clone(), "template" => .1.quote()))]
    NotFound(String, PathBuf),
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
    pub suffix: Option<OsString>,

    /// Whether to treat the template argument as a single file path component.
    pub treat_as_template: bool,

    /// The template to use for the name of the temporary file.
    pub template: OsString,
}

impl Options {
    fn from(matches: &ArgMatches) -> Self {
        let tmpdir = matches
            .get_one::<Option<PathBuf>>(OPT_TMPDIR)
            .or_else(|| matches.get_one::<Option<PathBuf>>(OPT_P))
            .map(|dir| match dir {
                // If the argument of -p/--tmpdir is non-empty, use it as the
                // tmpdir.
                Some(d) => d.clone(),
                // Otherwise use $TMPDIR if set, else use the system's default
                // temporary directory.
                None => get_tmpdir_env_or_default(),
            });
        let (tmpdir, template) = match matches.get_one::<OsString>(ARG_TEMPLATE) {
            // If no template argument is given, `--tmpdir` is implied.
            None => {
                let tmpdir = Some(tmpdir.unwrap_or_else(get_tmpdir_env_or_default));
                let template = DEFAULT_TEMPLATE;
                (tmpdir, OsString::from(template))
            }
            Some(template) => {
                let tmpdir = if let Some(tmpdir) = env::var_os(TMPDIR_ENV_VAR)
                    && matches.get_flag(OPT_T)
                {
                    Some(PathBuf::from(tmpdir))
                } else if tmpdir.is_some() {
                    tmpdir
                } else if matches.get_flag(OPT_T) || matches.contains_id(OPT_TMPDIR) {
                    // If --tmpdir is given without an argument, or -t is given
                    // export in TMPDIR
                    Some(env::temp_dir())
                } else {
                    None
                };
                (tmpdir, template.clone())
            }
        };
        Self {
            directory: matches.get_flag(OPT_DIRECTORY),
            dry_run: matches.get_flag(OPT_DRY_RUN),
            quiet: matches.get_flag(OPT_QUIET),
            tmpdir,
            suffix: matches.get_one::<OsString>(OPT_SUFFIX).cloned(),
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
    let bytes = s.as_bytes();

    // Find the index of the last 'X'.
    let end = bytes.iter().rposition(|&b| b == b'X')?;

    // Walk left to find the start of the run of Xs that ends at `end`.
    let mut start = end;
    while start > 0 && bytes[start - 1] == b'X' {
        start -= 1;
    }

    if end + 1 - start >= 3 {
        Some((start, end + 1))
    } else {
        None
    }
}

impl Params {
    fn from(options: Options) -> Result<Self, MkTempError> {
        // Convert OsString template to string for processing
        // When using -t flag, be permissive with invalid UTF-8 like GNU mktemp
        // Otherwise, maintain strict UTF-8 validation (existing behavior)
        let template_str = if options.treat_as_template {
            // For -t templates, use lossy conversion for GNU compatibility
            options.template.to_string_lossy().into_owned()
        } else {
            // For regular templates, maintain strict validation
            match options.template.to_str() {
                Some(s) => s.to_string(),
                None => {
                    return Err(MkTempError::InvalidTemplate(
                        "template contains invalid UTF-8".into(),
                    ));
                }
            }
        };

        // The template argument must end in 'X' if a suffix option is given.
        if options.suffix.is_some() && !template_str.ends_with('X') {
            return Err(MkTempError::MustEndInX(template_str.clone()));
        }

        // Get the start and end indices of the randomized part of the template.
        //
        // For example, if the template is "abcXXXXyz", then `i` is 3 and `j` is 7.
        let Some((i, j)) = find_last_contiguous_block_of_xs(&template_str) else {
            let s = match options.suffix {
                // If a suffix is specified, the error message includes the template without the suffix.
                Some(_) => template_str
                    .chars()
                    .take(template_str.len())
                    .collect::<String>(),
                None => template_str.clone(),
            };
            return Err(MkTempError::TooFewXs(s));
        };

        // Combine the directory given as an option and the prefix of the template.
        //
        // For example, if `tmpdir` is "a/b" and the template is "c/dXXX",
        // then `prefix` is "a/b/c/d".
        let tmpdir = options.tmpdir;
        let prefix_from_option = tmpdir.clone().unwrap_or_default();
        let prefix_from_template = &template_str[..i];
        let prefix_path = Path::new(&prefix_from_option).join(prefix_from_template);
        if options.treat_as_template && prefix_from_template.contains(MAIN_SEPARATOR) {
            return Err(MkTempError::PrefixContainsDirSeparator(
                template_str.clone(),
            ));
        }
        if tmpdir.is_some() && Path::new(prefix_from_template).is_absolute() {
            return Err(MkTempError::InvalidTemplate(template_str.clone().into()));
        }

        // Split the parent directory from the file part of the prefix.
        //
        // For example, if `prefix_path` is "a/b/c/d", then `directory` is
        // "a/b/c" and `prefix` gets reassigned to "d".
        let (directory, prefix) = {
            let prefix_str = prefix_path.to_string_lossy();
            if prefix_str.ends_with(MAIN_SEPARATOR) {
                (prefix_path, String::new())
            } else {
                let directory = match prefix_path.parent() {
                    None => PathBuf::new(),
                    Some(d) => d.to_path_buf(),
                };
                let prefix = match prefix_path.file_name() {
                    None => String::new(),
                    Some(f) => f.to_string_lossy().to_string(),
                };
                (directory, prefix)
            }
        };

        // Combine the suffix from the template with the suffix given as an option.
        //
        // For example, if the suffix command-line argument is ".txt" and
        // the template is "XXXabc", then `suffix` is "abc.txt".
        let suffix_from_option = options
            .suffix
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let suffix_from_template = &template_str[j..];
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
            directory,
            prefix,
            num_rand_chars,
            suffix,
        })
    }
}

/// Custom parser that converts empty string to `None`, and non-empty string to
/// `Some(PathBuf)`.
///
/// This parser is used for the `-p` and `--tmpdir` options where an empty string
/// argument should be treated as "not provided", causing mktemp to fall back to
/// using the `$TMPDIR` environment variable or the system's default temporary
/// directory.
///
/// # Examples
///
/// - Empty string `""` -> `None`
/// - Non-empty string `"/tmp"` -> `Some(PathBuf::from("/tmp"))`
///
/// This handles the special case where users can pass an empty directory name
/// to explicitly request fallback behavior.
#[derive(Clone, Debug)]
struct OptionalPathBufParser;

impl TypedValueParser for OptionalPathBufParser {
    type Value = Option<PathBuf>;

    fn parse_ref(
        &self,
        _cmd: &Command,
        _arg: Option<&Arg>,
        value: &OsStr,
    ) -> Result<Self::Value, clap::Error> {
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(PathBuf::from(value)))
        }
    }
}

impl ValueParserFactory for OptionalPathBufParser {
    type Parser = Self;

    fn value_parser() -> Self::Parser {
        Self
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args: Vec<_> = args.collect();
    let matches = match uu_app().try_get_matches_from(&args) {
        Ok(m) => m,
        Err(e) => {
            use uucore::clap_localization::handle_clap_error_with_exit_code;
            if e.kind() == clap::error::ErrorKind::UnknownArgument {
                handle_clap_error_with_exit_code(e, 1);
            }
            if e.kind() == clap::error::ErrorKind::TooManyValues
                && e.context().any(|(kind, val)| {
                    kind == clap::error::ContextKind::InvalidArg
                        && val == &clap::error::ContextValue::String("[template]".into())
                })
            {
                return Err(UUsageError::new(
                    1,
                    translate!("mktemp-error-too-many-templates"),
                ));
            }
            return Err(e.into());
        }
    };

    // Parse command-line options into a format suitable for the
    // application logic.
    let options = Options::from(&matches);

    if env::var_os("POSIXLY_CORRECT").is_some() {
        // If POSIXLY_CORRECT was set, template MUST be the last argument.
        if matches.contains_id(ARG_TEMPLATE) {
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
        Ok(dry_exec(&tmpdir, &prefix, rand, &suffix))
    } else {
        exec(&tmpdir, &prefix, rand, &suffix, make_dir)
    };

    let res = if suppress_file_err {
        // Mapping all UErrors to ExitCodes prevents the errors from being printed
        res.map_err(|e| e.code().into())
    } else {
        res
    };
    println_verbatim(res?).map_err_context(|| translate!("mktemp-error-failed-print"))
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("mktemp-about"))
        .override_usage(format_usage(&translate!("mktemp-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_DIRECTORY)
                .short('d')
                .long(OPT_DIRECTORY)
                .help(translate!("mktemp-help-directory"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_DRY_RUN)
                .short('u')
                .long(OPT_DRY_RUN)
                .help(translate!("mktemp-help-dry-run"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_QUIET)
                .short('q')
                .long("quiet")
                .help(translate!("mktemp-help-quiet"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_SUFFIX)
                .long(OPT_SUFFIX)
                .help(translate!("mktemp-help-suffix"))
                .value_name("SUFFIX")
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(OPT_P)
                .short('p')
                .help(translate!("mktemp-help-p"))
                .value_name("DIR")
                .num_args(1)
                .value_parser(OptionalPathBufParser)
                .value_hint(clap::ValueHint::DirPath),
        )
        .arg(
            Arg::new(OPT_TMPDIR)
                .long(OPT_TMPDIR)
                .help(translate!("mktemp-help-tmpdir"))
                .value_name("DIR")
                // Allows use of default argument just by setting --tmpdir. Else,
                // use provided input to generate tmpdir
                .num_args(0..=1)
                // Require an equals to avoid ambiguity if no tmpdir is supplied
                .require_equals(true)
                .overrides_with(OPT_P)
                .value_parser(OptionalPathBufParser)
                .value_hint(clap::ValueHint::DirPath),
        )
        .arg(
            Arg::new(OPT_T)
                .short('t')
                .help(translate!("mktemp-help-t"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(ARG_TEMPLATE)
                .num_args(..=1)
                .value_parser(clap::value_parser!(OsString)),
        )
}

fn dry_exec(tmpdir: &Path, prefix: &str, rand: usize, suffix: &str) -> PathBuf {
    let len = prefix.len() + suffix.len() + rand;
    let mut buf = Vec::with_capacity(len);
    buf.extend(prefix.as_bytes());
    buf.extend(iter::repeat_n(b'X', rand));
    buf.extend(suffix.as_bytes());

    // Randomize.
    let bytes = &mut buf[prefix.len()..prefix.len() + rand];
    SmallRng::try_from_os_rng()
        .unwrap_or_else(|_| {
            //rand::rng panics if getrandom failed
            SmallRng::seed_from_u64(bytes.as_ptr() as usize as u64)
        })
        .fill(bytes);
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
    Path::new(tmpdir).join(buf)
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

    // On *nix platforms grant read-write-execute for owner only.
    // The directory is created with these permission at creation time, using mkdir(3) syscall.
    // This is not relevant on Windows systems. See: https://docs.rs/tempfile/latest/tempfile/#security
    // `fs` is not imported on Windows anyways.
    #[cfg(not(windows))]
    builder.permissions(fs::Permissions::from_mode(0o700));

    match builder.tempdir_in(dir) {
        Ok(d) => {
            // `keep` consumes the TempDir without removing it
            let path = d.keep();
            Ok(path)
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let filename = format!("{prefix}{}{suffix}", "X".repeat(rand));
            let path = Path::new(dir).join(filename);
            Err(MkTempError::NotFound(translate!("mktemp-template-type-directory"), path).into())
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
            let filename = format!("{prefix}{}{suffix}", "X".repeat(rand));
            let path = Path::new(dir).join(filename);
            Err(MkTempError::NotFound(translate!("mktemp-template-type-file"), path).into())
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

/// Reads from `TMPDIR_ENV_VAR` but defaults to /tmp if value is set to empty string.
fn get_tmpdir_env_or_default() -> PathBuf {
    match env::var_os(TMPDIR_ENV_VAR) {
        Some(val) if val.is_empty() => PathBuf::from(FALLBACK_TMPDIR),
        _ => env::temp_dir(),
    }
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
        Ok(dry_exec(&tmpdir, &prefix, rand, &suffix))
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
