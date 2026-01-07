// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::super::Behavior;
use file_diff::diff;
use std::fs::{self, metadata};
use std::path::{MAIN_SEPARATOR, Path};
use uucore::error::UResult;
use uucore::{show_error, translate};

pub(crate) fn platform_umask() -> u32 {
    0
}

pub(crate) fn resolve_owner(owner: &str) -> UResult<Option<u32>> {
    if owner.is_empty() {
        Ok(None)
    } else {
        show_error!(
            "{}",
            translate!("install-error-option-unsupported", "option" => "--owner")
        );
        Err(1.into())
    }
}

pub(crate) fn resolve_group(group: &str) -> UResult<Option<u32>> {
    if group.is_empty() {
        Ok(None)
    } else {
        show_error!(
            "{}",
            translate!("install-error-option-unsupported", "option" => "--group")
        );
        Err(1.into())
    }
}

pub(crate) fn is_potential_directory_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    path_str.ends_with(MAIN_SEPARATOR) || path_str.ends_with('/') || path.is_dir()
}

pub(crate) fn chown_optional_user_group(_path: &Path, _b: &Behavior) -> UResult<()> {
    Ok(())
}

pub(crate) fn need_copy(from: &Path, to: &Path, _b: &Behavior) -> bool {
    let Ok(from_meta) = metadata(from) else {
        return true;
    };

    let Ok(to_meta) = metadata(to) else {
        return true;
    };

    if let Ok(to_symlink_meta) = fs::symlink_metadata(to) {
        if to_symlink_meta.file_type().is_symlink() {
            return true;
        }
    }

    if !from_meta.is_file() || !to_meta.is_file() {
        return true;
    }

    if from_meta.len() != to_meta.len() {
        return true;
    }

    if !diff(&from.to_string_lossy(), &to.to_string_lossy()) {
        return true;
    }

    false
}
