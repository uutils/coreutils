// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::mem::offset_of;

#[cfg(target_env = "gnu")]
use libc::__UT_NAMESIZE as UT_NAMESIZE;
#[cfg(not(target_env = "gnu"))]
use libc::UT_NAMESIZE;

// Build a native Linux user record for unit and integration tests.
pub(crate) fn utmpx_record(user: &str) -> Vec<u8> {
    let mut bytes = vec![0; size_of::<libc::utmpx>()];
    let record_type = libc::USER_PROCESS.to_ne_bytes();
    let type_offset = offset_of!(libc::utmpx, ut_type);
    bytes[type_offset..type_offset + record_type.len()].copy_from_slice(&record_type);
    let user_offset = offset_of!(libc::utmpx, ut_user);
    assert!(user.len() <= UT_NAMESIZE);
    bytes[user_offset..user_offset + user.len()].copy_from_slice(user.as_bytes());
    bytes
}
