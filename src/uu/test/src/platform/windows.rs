// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) PSECURITY PSID

use std::ffi::OsStr;
use std::ptr;
use uucore::wide::ToWide;
use windows_sys::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, HANDLE, LocalFree};
use windows_sys::Win32::Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT};
use windows_sys::Win32::Security::{
    EqualSid, GROUP_SECURITY_INFORMATION, GetTokenInformation, OWNER_SECURITY_INFORMATION,
    PSECURITY_DESCRIPTOR, PSID, TOKEN_QUERY, TokenOwner, TokenPrimaryGroup,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// A security descriptor allocated by the security API, freed on drop.
struct Descriptor(PSECURITY_DESCRIPTOR);

impl Drop for Descriptor {
    fn drop(&mut self) {
        // SAFETY: the pointer comes from GetNamedSecurityInfoW; freeing a null
        // pointer is a no-op.
        unsafe { LocalFree(self.0) };
    }
}

/// A handle on the token of the current process, closed on drop.
struct Token(HANDLE);

impl Drop for Token {
    fn drop(&mut self) {
        // SAFETY: the handle comes from a successful OpenProcessToken.
        unsafe { CloseHandle(self.0) };
    }
}

/// Whether `sid` is the SID the current process token stamps onto the objects it
/// creates: its owner or, with `group`, its primary group.
///
/// Those two are the counterparts of the effective UID and GID. They are not
/// always the token user: a process running elevated has the Administrators
/// group as its token owner, and that is what lands in the descriptor of a file
/// it creates, so comparing against the token user would report that you do not
/// own a file you just created. Group membership is not a substitute — it would
/// also match every group you happen to belong to, such as `Everyone`, and
/// report ownership of files that are not yours.
fn matches_token_sid(sid: PSID, group: bool) -> bool {
    let mut handle: HANDLE = ptr::null_mut();
    // SAFETY: GetCurrentProcess returns a pseudo handle that needs no closing,
    // and `handle` is a valid out pointer.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut handle) } == 0 {
        return false;
    }
    let token = Token(handle);
    let class = if group { TokenPrimaryGroup } else { TokenOwner };

    let mut size = 0;
    // SAFETY: a null buffer of length zero only asks for the size to allocate.
    unsafe { GetTokenInformation(token.0, class, ptr::null_mut(), 0, &raw mut size) };
    if size == 0 {
        return false;
    }

    // TOKEN_OWNER and TOKEN_PRIMARY_GROUP are both a lone SID pointer followed
    // by the SID it points at, so the buffer has to be pointer aligned.
    let mut buffer = vec![0usize; (size as usize).div_ceil(size_of::<usize>())];
    // SAFETY: `buffer` is at least `size` bytes long, as reported above.
    let ok = unsafe {
        GetTokenInformation(
            token.0,
            class,
            buffer.as_mut_ptr().cast(),
            size,
            &raw mut size,
        )
    };
    if ok == 0 {
        return false;
    }

    // SAFETY: the call above wrote one of those two structures into `buffer`,
    // and the SID pointer is its first field.
    let token_sid = unsafe { *buffer.as_ptr().cast::<PSID>() };

    // SAFETY: `sid` is valid while its descriptor lives, `token_sid` while
    // `buffer` does.
    !token_sid.is_null() && unsafe { EqualSid(sid, token_sid) } != 0
}

/// Whether `path` is owned by the current process token, comparing its owner
/// (or, with `group`, its primary group) SID — the Windows analogue of matching
/// `st_uid`/`st_gid` against the effective UID/GID for `-O` and `-G`.
pub fn owned_by_current_token(path: &OsStr, group: bool) -> bool {
    let wide_path = path.to_wide_null();
    let mut sid: PSID = ptr::null_mut();
    // Owns the memory `sid` points into, so it has to live until the comparison
    // below is done.
    let mut descriptor = Descriptor(ptr::null_mut());
    let (info, owner_out, group_out) = if group {
        (GROUP_SECURITY_INFORMATION, ptr::null_mut(), &raw mut sid)
    } else {
        (OWNER_SECURITY_INFORMATION, &raw mut sid, ptr::null_mut())
    };

    // SAFETY: `wide_path` is NUL-terminated and outlives the call, and the out
    // pointers are valid. On success `sid` points into the descriptor.
    let status = unsafe {
        GetNamedSecurityInfoW(
            wide_path.as_ptr(),
            SE_FILE_OBJECT,
            info,
            owner_out,
            group_out,
            ptr::null_mut(),
            ptr::null_mut(),
            &raw mut descriptor.0,
        )
    };

    status == ERROR_SUCCESS && !sid.is_null() && matches_token_sid(sid, group)
}
