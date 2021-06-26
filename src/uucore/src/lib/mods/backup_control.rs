use std::{
    env,
    path::{Path, PathBuf},
};

pub static BACKUP_CONTROL_VALUES: &[&str] = &[
    "simple", "never", "numbered", "t", "existing", "nil", "none", "off",
];

pub static BACKUP_CONTROL_LONG_HELP: &str =
"The backup suffix is '~', unless set with --suffix or SIMPLE_BACKUP_SUFFIX.
The version control method may be selected via the --backup option or through
the VERSION_CONTROL environment variable.  Here are the values:

  none, off       never make backups (even if --backup is given)
  numbered, t     make numbered backups
  existing, nil   numbered if numbered backups exist, simple otherwise
  simple, never   always make simple backups";

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BackupMode {
    NoBackup,
    SimpleBackup,
    NumberedBackup,
    ExistingBackup,
}

pub fn determine_backup_suffix(supplied_suffix: Option<&str>) -> String {
    if let Some(suffix) = supplied_suffix {
        String::from(suffix)
    } else {
        env::var("SIMPLE_BACKUP_SUFFIX").unwrap_or_else(|_| "~".to_owned())
    }
}

/// # TODO
///
/// This function currently deviates slightly from how the [manual][1] describes
/// that it should work. In particular, the current implementation:
///
/// 1. Doesn't strictly respect the order in which to determine the backup type,
///    which is (in order of precedence)
///     1. Take a valid value to the '--backup' option
///     2. Take the value of the `VERSION_CONTROL` env var
///     3. default to 'existing'
/// 2. Doesn't accept abbreviations to the 'backup_option' parameter
///
/// [1]: https://www.gnu.org/software/coreutils/manual/html_node/Backup-options.html
pub fn determine_backup_mode(backup_opt_exists: bool, backup_opt: Option<&str>) -> BackupMode {
    if backup_opt_exists {
        match backup_opt.map(String::from) {
            // default is existing, see:
            // https://www.gnu.org/software/coreutils/manual/html_node/Backup-options.html
            None => BackupMode::ExistingBackup,
            Some(mode) => match &mode[..] {
                "simple" | "never" => BackupMode::SimpleBackup,
                "numbered" | "t" => BackupMode::NumberedBackup,
                "existing" | "nil" => BackupMode::ExistingBackup,
                "none" | "off" => BackupMode::NoBackup,
                _ => panic!(), // cannot happen as it is managed by clap
            },
        }
    } else {
        BackupMode::NoBackup
    }
}

/// Match a backup option string to a `BackupMode`.
///
/// The GNU manual specifies that abbreviations to options are valid as long as
/// they aren't ambiguous. This function matches the given `method` argument
/// against all valid backup options (via `starts_with`), and returns a valid
/// [`BackupMode`] if exactly one backup option matches the `method` given.
///
/// `origin` is required in order to format the generated error message
/// properly, when an error occurs.
///
///
/// # Errors
///
/// If `method` is ambiguous (i.e. may resolve to multiple backup modes) or
/// invalid, an error is returned. The error contains the formatted error string
/// which may then be passed to the [`show_usage_error`] macro.
fn match_method(method: &str, origin: &str) -> Result<BackupMode, String> {
    let x = vec!["simple", "never", "numbered", "t",
                 "existing", "nil", "none", "off"];

    let matches: Vec<&&str> = x.iter()
        .filter(|val| val.starts_with(method))
        .collect();
    if matches.len() == 1 {
        match *matches[0] {
            "simple" | "never" => Ok(BackupMode::SimpleBackup),
            "numbered" | "t" => Ok(BackupMode::NumberedBackup),
            "existing" | "nil" => Ok(BackupMode::ExistingBackup),
            "none" | "off" => Ok(BackupMode::NoBackup),
            _ => panic!(),  // cannot happen as we must have exactly one match
                            // from the list above.
        }
    } else {
        let error_type = if matches.len() == 0 { "invalid" } else { "ambiguous" };
        Err(format!(
"{0} argument ‘{1}’ for ‘{2}’
Valid arguments are:
  - ‘none’, ‘off’
  - ‘simple’, ‘never’
  - ‘existing’, ‘nil’
  - ‘numbered’, ‘t’", error_type, method, origin))
    }
}

pub fn get_backup_path(
    backup_mode: BackupMode,
    backup_path: &Path,
    suffix: &str,
) -> Option<PathBuf> {
    match backup_mode {
        BackupMode::NoBackup => None,
        BackupMode::SimpleBackup => Some(simple_backup_path(backup_path, suffix)),
        BackupMode::NumberedBackup => Some(numbered_backup_path(backup_path)),
        BackupMode::ExistingBackup => Some(existing_backup_path(backup_path, suffix)),
    }
}

fn simple_backup_path(path: &Path, suffix: &str) -> PathBuf {
    let mut p = path.to_string_lossy().into_owned();
    p.push_str(suffix);
    PathBuf::from(p)
}

fn numbered_backup_path(path: &Path) -> PathBuf {
    for i in 1_u64.. {
        let path_str = &format!("{}.~{}~", path.to_string_lossy(), i);
        let path = Path::new(path_str);
        if !path.exists() {
            return path.to_path_buf();
        }
    }
    panic!("cannot create backup")
}

fn existing_backup_path(path: &Path, suffix: &str) -> PathBuf {
    let test_path_str = &format!("{}.~1~", path.to_string_lossy());
    let test_path = Path::new(test_path_str);
    if test_path.exists() {
        numbered_backup_path(path)
    } else {
        simple_backup_path(path, suffix)
    }
}
