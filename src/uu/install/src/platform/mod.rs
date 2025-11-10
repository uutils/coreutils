// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(unix)]
pub(crate) use self::unix::{
    chown_optional_user_group, is_potential_directory_path, need_copy, platform_umask,
    resolve_group, resolve_owner,
};

#[cfg(not(unix))]
pub(crate) use self::non_unix::{
    chown_optional_user_group, is_potential_directory_path, need_copy, platform_umask,
    resolve_group, resolve_owner,
};

#[cfg(unix)]
mod unix;

#[cfg(not(unix))]
mod non_unix;
