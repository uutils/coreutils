// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::super::{Behavior, InstallError};
use file_diff::diff;
use std::fs::{self, metadata};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{MAIN_SEPARATOR, Path};
use uucore::entries::{grp2gid, usr2uid};
use uucore::error::UResult;
use uucore::mode::get_umask;
use uucore::perms::{Verbosity, VerbosityLevel, wrap_chown};
use uucore::process::{getegid, geteuid};

#[cfg(feature = "selinux")]
use uucore::selinux::contexts_differ;

pub(crate) fn platform_umask() -> u32 {
    get_umask()
}

pub(crate) fn resolve_owner(owner: &str) -> UResult<Option<u32>> {
    if owner.is_empty() {
        Ok(None)
    } else {
        usr2uid(owner)
            .map(Some)
            .map_err(|_| InstallError::InvalidUser(owner.to_string()).into())
    }
}

pub(crate) fn resolve_group(group: &str) -> UResult<Option<u32>> {
    if group.is_empty() {
        Ok(None)
    } else {
        grp2gid(group)
            .map(Some)
            .map_err(|_| InstallError::InvalidGroup(group.to_string()).into())
    }
}

pub(crate) fn is_potential_directory_path(path: &Path) -> bool {
    let separator = MAIN_SEPARATOR as u8;
    path.as_os_str().as_bytes().last() == Some(&separator) || path.is_dir()
}

pub(crate) fn chown_optional_user_group(path: &Path, b: &Behavior) -> UResult<()> {
    let verbosity = Verbosity {
        groups_only: b.owner_id.is_none(),
        level: VerbosityLevel::Normal,
    };

    let (owner_id, group_id) = if b.owner_id.is_some() || b.group_id.is_some() {
        (b.owner_id, b.group_id)
    } else if geteuid() == 0 {
        (Some(0), Some(0))
    } else {
        return Ok(());
    };

    let meta = match metadata(path) {
        Ok(meta) => meta,
        Err(e) => return Err(InstallError::MetadataFailed(e).into()),
    };
    match wrap_chown(path, &meta, owner_id, group_id, false, verbosity) {
        Ok(msg) if b.verbose && !msg.is_empty() => println!("chown: {msg}"),
        Ok(_) => {}
        Err(e) => return Err(InstallError::ChownFailed(path.to_path_buf(), e).into()),
    }

    Ok(())
}

pub(crate) fn need_copy(from: &Path, to: &Path, b: &Behavior) -> bool {
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

    let extra_mode: u32 = 0o7000;
    let all_modes: u32 = 0o7777;

    if b.mode() & extra_mode != 0
        || from_meta.mode() & extra_mode != 0
        || to_meta.mode() & extra_mode != 0
    {
        return true;
    }

    if b.mode() != to_meta.mode() & all_modes {
        return true;
    }

    if !from_meta.is_file() || !to_meta.is_file() {
        return true;
    }

    if from_meta.len() != to_meta.len() {
        return true;
    }

    #[cfg(feature = "selinux")]
    if !b.unprivileged && b.preserve_context && contexts_differ(from, to) {
        return true;
    }

    if let Some(owner_id) = b.owner_id {
        if !b.unprivileged && owner_id != to_meta.uid() {
            return true;
        }
    }

    if let Some(group_id) = b.group_id {
        if !b.unprivileged && group_id != to_meta.gid() {
            return true;
        }
    } else if !b.unprivileged && needs_copy_for_ownership(to, &to_meta) {
        return true;
    }

    if !diff(&from.to_string_lossy(), &to.to_string_lossy()) {
        return true;
    }

    false
}

fn needs_copy_for_ownership(to: &Path, to_meta: &fs::Metadata) -> bool {
    if to_meta.uid() != geteuid() {
        return true;
    }

    let expected_gid = to
        .parent()
        .and_then(|parent| metadata(parent).ok())
        .filter(|parent_meta| parent_meta.mode() & 0o2000 != 0)
        .map_or(getegid(), |parent_meta| parent_meta.gid());

    to_meta.gid() != expected_gid
}
