// This file is part of the uutils coreutils package.
//
// (c) Sunrin SHIMURA
// Collaborator: Jian Zeng
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (paths) GPGHome

use clap::{crate_version, Arg, Command};
use uucore::display::{println_verbatim, Quotable};
use uucore::error::{FromIo, UError, UResult};
use uucore::format_usage;

use std::env;
use std::error::Error;
use std::fmt::Display;
use std::iter;
use std::path::{is_separator, Path, PathBuf};

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
}

impl UError for MkTempError {}

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
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let template = matches.value_of(ARG_TEMPLATE).unwrap();
    let tmpdir = matches.value_of(OPT_TMPDIR).unwrap_or_default();

    // Treat the template string as a path to get the directory
    // containing the last component.
    let path = PathBuf::from(template);

    let (template, tmpdir) = if matches.is_present(OPT_TMPDIR)
        && !PathBuf::from(tmpdir).is_dir() // if a temp dir is provided, it must be an actual path
        && tmpdir.contains("XXX")
    // If this is a template, it has to contain at least 3 X
        && template == DEFAULT_TEMPLATE
    // That means that clap does not think we provided a template
    {
        // Special case to workaround a limitation of clap when doing
        // mktemp --tmpdir apt-key-gpghome.XXX
        // The behavior should be
        // mktemp --tmpdir $TMPDIR apt-key-gpghome.XX
        // As --tmpdir is empty
        //
        // Fixed in clap 3
        // See https://github.com/clap-rs/clap/pull/1587
        let tmp = env::temp_dir();
        (tmpdir, tmp)
    } else if !matches.is_present(OPT_TMPDIR) {
        // In this case, the command line was `mktemp -t XXX`, so we
        // treat the argument `XXX` as though it were a filename
        // regardless of whether it has path separators in it.
        if matches.is_present(OPT_T) {
            let tmp = env::temp_dir();
            (template, tmp)
        // In this case, the command line was `mktemp XXX`, so we need
        // to parse out the parent directory and the filename from the
        // argument `XXX`, since it may be include path separators.
        } else {
            let tmp = match path.parent() {
                None => PathBuf::from("."),
                Some(d) => PathBuf::from(d),
            };
            let filename = path.file_name();
            let template = filename.unwrap().to_str().unwrap();
            (template, tmp)
        }
    } else {
        (template, PathBuf::from(tmpdir))
    };

    let make_dir = matches.is_present(OPT_DIRECTORY);
    let dry_run = matches.is_present(OPT_DRY_RUN);
    let suppress_file_err = matches.is_present(OPT_QUIET);

    let (prefix, rand, suffix) = parse_template(template, matches.value_of(OPT_SUFFIX))?;

    if matches.is_present(OPT_TMPDIR) && PathBuf::from(prefix).is_absolute() {
        return Err(MkTempError::InvalidTemplate(template.into()).into());
    }

    let res = if dry_run {
        dry_exec(tmpdir, prefix, rand, suffix)
    } else {
        exec(&tmpdir, prefix, rand, suffix, make_dir)
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
                .value_name("DIR"),
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
                .default_value(DEFAULT_TEMPLATE),
        )
}

/// Parse a template string into prefix, suffix, and random components.
///
/// `temp` is the template string, with three or more consecutive `X`s
/// representing a placeholder for randomly generated characters (for
/// example, `"abc_XXX.txt"`). If `temp` ends in an `X`, then a suffix
/// can be specified by `suffix` instead.
///
/// # Errors
///
/// * If there are fewer than three consecutive `X`s in `temp`.
/// * If `suffix` is a [`Some`] object but `temp` does not end in `X`.
/// * If the suffix (specified either way) contains a path separator.
///
/// # Examples
///
/// ```rust,ignore
/// assert_eq!(parse_template("XXX", None).unwrap(), ("", 3, ""));
/// assert_eq!(parse_template("abcXXX", None).unwrap(), ("abc", 3, ""));
/// assert_eq!(parse_template("XXXdef", None).unwrap(), ("", 3, "def"));
/// assert_eq!(parse_template("abcXXXdef", None).unwrap(), ("abc", 3, "def"));
/// ```
fn parse_template<'a>(
    temp: &'a str,
    suffix: Option<&'a str>,
) -> Result<(&'a str, usize, &'a str), MkTempError> {
    let right = match temp.rfind('X') {
        Some(r) => r + 1,
        None => return Err(MkTempError::TooFewXs(temp.into())),
    };
    let left = temp[..right].rfind(|c| c != 'X').map_or(0, |i| i + 1);
    let prefix = &temp[..left];
    let rand = right - left;

    if rand < 3 {
        return Err(MkTempError::TooFewXs(temp.into()));
    }

    let mut suf = &temp[right..];

    if let Some(s) = suffix {
        if suf.is_empty() {
            suf = s;
        } else {
            return Err(MkTempError::MustEndInX(temp.into()));
        }
    };

    if prefix.chars().any(is_separator) {
        return Err(MkTempError::PrefixContainsDirSeparator(temp.into()));
    }

    if suf.chars().any(is_separator) {
        return Err(MkTempError::SuffixContainsDirSeparator(suf.into()));
    }

    Ok((prefix, rand, suf))
}

pub fn dry_exec(mut tmpdir: PathBuf, prefix: &str, rand: usize, suffix: &str) -> UResult<()> {
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
    tmpdir.push(buf);
    println_verbatim(tmpdir).map_err_context(|| "failed to print directory name".to_owned())
}

fn exec(dir: &Path, prefix: &str, rand: usize, suffix: &str, make_dir: bool) -> UResult<()> {
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

    // Get just the last component of the path to the created
    // temporary file or directory.
    let filename = path.file_name();
    let filename = filename.unwrap().to_str().unwrap();

    // Join the directory to the path to get the path to print. We
    // cannot use the path returned by the `Builder` because it gives
    // the absolute path and we need to return a filename that matches
    // the template given on the command-line which might be a
    // relative path.
    let mut path = dir.to_path_buf();
    path.push(filename);

    println_verbatim(path).map_err_context(|| "failed to print directory name".to_owned())
}

#[cfg(test)]
mod tests {
    use crate::parse_template;

    #[test]
    fn test_parse_template_no_suffix() {
        assert_eq!(parse_template("XXX", None).unwrap(), ("", 3, ""));
        assert_eq!(parse_template("abcXXX", None).unwrap(), ("abc", 3, ""));
        assert_eq!(parse_template("XXXdef", None).unwrap(), ("", 3, "def"));
        assert_eq!(
            parse_template("abcXXXdef", None).unwrap(),
            ("abc", 3, "def")
        );
    }

    #[test]
    fn test_parse_template_suffix() {
        assert_eq!(parse_template("XXX", Some("def")).unwrap(), ("", 3, "def"));
        assert_eq!(
            parse_template("abcXXX", Some("def")).unwrap(),
            ("abc", 3, "def")
        );
    }

    #[test]
    fn test_parse_template_errors() {
        assert!(parse_template("a/bXXX", None).is_err());
        assert!(parse_template("XXXa/b", None).is_err());
        assert!(parse_template("XX", None).is_err());
        assert!(parse_template("XXXabc", Some("def")).is_err());
        assert!(parse_template("XXX", Some("a/b")).is_err());
    }
}
