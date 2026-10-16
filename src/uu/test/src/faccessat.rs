// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) egid euid accessat EACCESS OPNOTSUPP NOSYS XOTH XGRP XUSR

use rustix::fs::Access;
use rustix::io;
#[cfg(not(any(windows, target_os = "wasi")))]
use rustix::process::{getegid, geteuid};
use std::ffi::OsStr;
use std::fs::OpenOptions;

/// Check `-r`, `-w` and `-x` with the process's effective credentials.
///
/// Let the OS account for supplementary groups, ACLs and privileged users.
#[cfg(not(any(windows, target_os = "wasi")))]
pub(crate) fn effective_access(path: &OsStr, access: Access) -> bool {
    use rustix::fs::{AtFlags, CWD, accessat};

    #[cfg(target_os = "android")]
    let flags = AtFlags::empty();

    #[cfg(not(target_os = "android"))]
    let flags = AtFlags::EACCESS;

    match accessat(CWD, path, access, flags) {
        Ok(()) => true,
        Err(io::Errno::NOSYS | io::Errno::OPNOTSUPP) => effective_access_fallback(path, access),
        Err(_) => false,
    }
}

fn effective_access_fallback(path: &OsStr, access: Access) -> bool {
    use rustix::fs::Mode;
    use rustix::fs::stat;
    use rustix::process::getgroups;
    if access.contains(Access::EXEC_OK) {
        let Ok(st) = stat(path) else {
            return false;
        };

        let mode = Mode::from_raw_mode(st.st_mode);
        let euid = geteuid().as_raw();
        let egid = getegid().as_raw();

        let executable = if euid == 0 {
            mode.intersects(Mode::XUSR | Mode::XGRP | Mode::XOTH)
        } else if st.st_uid == euid {
            mode.contains(Mode::XUSR)
        } else {
            let in_group = st.st_gid == egid
                || getgroups()
                    .is_ok_and(|groups| groups.iter().any(|gid| gid.as_raw() == st.st_gid));

            if in_group {
                mode.contains(Mode::XGRP)
            } else {
                mode.contains(Mode::XOTH)
            }
        };

        if !executable {
            return false;
        }
    }

    if access.intersects(Access::READ_OK | Access::WRITE_OK) {
        let mut options = OpenOptions::new();

        options
            .read(access.contains(Access::READ_OK))
            .write(access.contains(Access::WRITE_OK));

        if options.open(path).is_err() {
            return false;
        }
    }

    if access == Access::EXISTS {
        return stat(path).is_ok();
    }

    true
}
