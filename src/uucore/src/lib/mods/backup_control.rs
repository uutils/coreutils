use std::{
    env,
    path::{Path, PathBuf},
};

pub static BACKUP_CONTROL_VALUES: &[&str] = &[
    "simple", "never", "numbered", "t", "existing", "nil", "none", "off",
];

pub static BACKUP_CONTROL_LONG_HELP: &str = "The backup suffix is '~', unless set with --suffix or SIMPLE_BACKUP_SUFFIX. Here are the version control values:

none, off
    never make backups (even if --backup is given)

numbered, t
    make numbered backups

existing, nil
    numbered if numbered backups exist, simple otherwise

simple, never
    always make simple backups";

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
        env::var("SIMPLE_BACKUP_SUFFIX").unwrap_or("~".to_owned())
    }
}

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

pub fn simple_backup_path(path: &Path, suffix: &str) -> PathBuf {
    let mut p = path.to_string_lossy().into_owned();
    p.push_str(suffix);
    PathBuf::from(p)
}

pub fn numbered_backup_path(path: &Path) -> PathBuf {
    for i in 1_u64.. {
        let path_str = &format!("{}.~{}~", path.to_string_lossy(), i);
        let path = Path::new(path_str);
        if !path.exists() {
            return path.to_path_buf();
        }
    }
    panic!("cannot create backup")
}

pub fn existing_backup_path(path: &Path, suffix: &str) -> PathBuf {
    let test_path_str = &format!("{}.~1~", path.to_string_lossy());
    let test_path = Path::new(test_path_str);
    if test_path.exists() {
        numbered_backup_path(path)
    } else {
        simple_backup_path(path, suffix)
    }
}
