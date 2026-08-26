// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(target_os = "redox")]
mod redox;

#[cfg(target_os = "redox")]
pub(super) use redox::{copy_special_file, rename_special_fallback, replace_symlink};
