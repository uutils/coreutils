// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) DACL PSECURITY PSID

use std::ffi::OsStr;
use std::fs::{Metadata, OpenOptions};
use std::io::{self, IsTerminal};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::{AsHandle, OwnedHandle};
use std::path::Path;
use uucore::fs::{FileInformation, infos_refer_to_same_file};
use windows_sys::Win32::Foundation::ERROR_ACCESS_DENIED;
use windows_sys::Win32::Security::{
    DACL_SECURITY_INFORMATION, GENERIC_MAPPING, GROUP_SECURITY_INFORMATION,
    OWNER_SECURITY_INFORMATION, SecurityImpersonation, TOKEN_DUPLICATE, TOKEN_QUERY,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ALL_ACCESS, FILE_FLAG_BACKUP_SEMANTICS, FILE_GENERIC_EXECUTE, FILE_GENERIC_READ,
    FILE_GENERIC_WRITE,
};

/// Confines the unsafe security calls: results come back as [`io::Error`] and
/// SIDs only as [`Sid`]s borrowed from the buffer that owns them.
mod sys {
    use std::ffi::OsStr;
    use std::io;
    use std::marker::PhantomData;
    use std::os::windows::io::{AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle};
    use std::ptr;
    use uucore::wide::ToWide;
    use windows_sys::Win32::Foundation::{ERROR_SUCCESS, HANDLE, LocalFree};
    use windows_sys::Win32::Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT};
    use windows_sys::Win32::Security::{
        AccessCheck, DuplicateToken, EqualSid, GENERIC_MAPPING, GROUP_SECURITY_INFORMATION,
        GetTokenInformation, MapGenericMask, OBJECT_SECURITY_INFORMATION,
        OWNER_SECURITY_INFORMATION, PRIVILEGE_SET, PSECURITY_DESCRIPTOR, PSID,
        SECURITY_IMPERSONATION_LEVEL, TOKEN_ACCESS_MASK, TokenOwner, TokenPrimaryGroup,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    /// A SID borrowed from the structure that owns its memory.
    #[derive(Clone, Copy)]
    pub struct Sid<'a>(PSID, PhantomData<&'a ()>);

    impl PartialEq for Sid<'_> {
        fn eq(&self, other: &Self) -> bool {
            // SAFETY: each SID is valid for as long as its owner lives, which
            // the borrows guarantee.
            unsafe { EqualSid(self.0, other.0) != 0 }
        }
    }

    /// A security descriptor allocated by the security API, freed on drop,
    /// along with the owner and group SIDs that point into it.
    pub struct SecurityDescriptor {
        ptr: PSECURITY_DESCRIPTOR,
        owner: PSID,
        group: PSID,
    }

    impl Drop for SecurityDescriptor {
        fn drop(&mut self) {
            // SAFETY: the pointer comes from GetNamedSecurityInfoW; freeing a
            // null pointer is a no-op.
            unsafe { LocalFree(self.ptr) };
        }
    }

    impl SecurityDescriptor {
        pub fn owner(&self) -> Option<Sid<'_>> {
            (!self.owner.is_null()).then_some(Sid(self.owner, PhantomData))
        }

        pub fn group(&self) -> Option<Sid<'_>> {
            (!self.group.is_null()).then_some(Sid(self.group, PhantomData))
        }
    }

    pub fn named_security_info(
        path: &OsStr,
        info: OBJECT_SECURITY_INFORMATION,
    ) -> io::Result<SecurityDescriptor> {
        let wide_path = path.to_wide_null();
        let mut descriptor = SecurityDescriptor {
            ptr: ptr::null_mut(),
            owner: ptr::null_mut(),
            group: ptr::null_mut(),
        };
        let owner = if info & OWNER_SECURITY_INFORMATION == 0 {
            ptr::null_mut()
        } else {
            &raw mut descriptor.owner
        };
        let group = if info & GROUP_SECURITY_INFORMATION == 0 {
            ptr::null_mut()
        } else {
            &raw mut descriptor.group
        };
        // SAFETY: `wide_path` is NUL-terminated and outlives the call, and each
        // out pointer is either valid or null.
        let status = unsafe {
            GetNamedSecurityInfoW(
                wide_path.as_ptr(),
                SE_FILE_OBJECT,
                info,
                owner,
                group,
                ptr::null_mut(),
                ptr::null_mut(),
                &raw mut descriptor.ptr,
            )
        };
        if status == ERROR_SUCCESS {
            Ok(descriptor)
        } else {
            Err(io::Error::from_raw_os_error(status as i32))
        }
    }

    pub fn open_process_token(access: TOKEN_ACCESS_MASK) -> io::Result<OwnedHandle> {
        let mut handle: HANDLE = ptr::null_mut();
        // SAFETY: GetCurrentProcess returns a pseudo handle that needs no
        // closing, and `handle` is a valid out pointer.
        if unsafe { OpenProcessToken(GetCurrentProcess(), access, &raw mut handle) } == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: OpenProcessToken succeeded, so `handle` is a fresh owned handle.
        Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
    }

    pub fn duplicate_token(
        token: BorrowedHandle,
        level: SECURITY_IMPERSONATION_LEVEL,
    ) -> io::Result<OwnedHandle> {
        let mut handle: HANDLE = ptr::null_mut();
        // SAFETY: `token` is a valid handle and `handle` a valid out pointer.
        if unsafe { DuplicateToken(token.as_raw_handle(), level, &raw mut handle) } == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: DuplicateToken succeeded, so `handle` is a fresh owned handle.
        Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
    }

    #[derive(Clone, Copy)]
    pub enum TokenSidClass {
        Owner,
        PrimaryGroup,
    }

    /// A SID of a token, kept in the buffer it points into.
    pub struct TokenSid(Vec<usize>);

    impl TokenSid {
        pub fn sid(&self) -> Sid<'_> {
            // SAFETY: GetTokenInformation filled the buffer with a structure
            // whose first field is the SID pointer.
            Sid(unsafe { *self.0.as_ptr().cast::<PSID>() }, PhantomData)
        }
    }

    pub fn token_sid(token: BorrowedHandle, class: TokenSidClass) -> io::Result<TokenSid> {
        let class = match class {
            TokenSidClass::Owner => TokenOwner,
            TokenSidClass::PrimaryGroup => TokenPrimaryGroup,
        };
        let token = token.as_raw_handle();

        let mut size = 0;
        // SAFETY: a null buffer of length zero only asks for the size to
        // allocate.
        unsafe { GetTokenInformation(token, class, ptr::null_mut(), 0, &raw mut size) };
        if size == 0 {
            return Err(io::Error::last_os_error());
        }

        // TOKEN_OWNER and TOKEN_PRIMARY_GROUP are a lone SID pointer followed
        // by the SID it points at, so the buffer has to be pointer aligned.
        let mut buffer = vec![0usize; (size as usize).div_ceil(size_of::<usize>())];
        // SAFETY: `buffer` is at least `size` bytes long, as reported above.
        let ok = unsafe {
            GetTokenInformation(
                token,
                class,
                buffer.as_mut_ptr().cast(),
                size,
                &raw mut size,
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(TokenSid(buffer))
    }

    pub fn access_check(
        descriptor: &SecurityDescriptor,
        token: BorrowedHandle,
        mut desired: u32,
        mapping: &GENERIC_MAPPING,
    ) -> io::Result<bool> {
        // SAFETY: both pointers refer to live locals.
        unsafe { MapGenericMask(&raw mut desired, mapping) };

        // One entry is enough: no file right is ever granted by a privilege.
        let mut privileges = PRIVILEGE_SET::default();
        let mut privileges_len = size_of::<PRIVILEGE_SET>() as u32;
        let mut granted = 0;
        let mut status = 0;
        // SAFETY: the descriptor and token are valid, every out pointer refers
        // to a live local, and `privileges_len` is the size of `privileges`.
        let ok = unsafe {
            AccessCheck(
                descriptor.ptr,
                token.as_raw_handle(),
                desired,
                mapping,
                &raw mut privileges,
                &raw mut privileges_len,
                &raw mut granted,
                &raw mut status,
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(status != 0)
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
fn matches_token_sid(sid: sys::Sid<'_>, group: bool) -> bool {
    let class = if group {
        sys::TokenSidClass::PrimaryGroup
    } else {
        sys::TokenSidClass::Owner
    };
    sys::open_process_token(TOKEN_QUERY)
        .and_then(|token| sys::token_sid(token.as_handle(), class))
        .is_ok_and(|token_sid| token_sid.sid() == sid)
}

/// Whether `path` is owned by the current process token, comparing its owner
/// (or, with `group`, its primary group) SID — the Windows analogue of matching
/// `st_uid`/`st_gid` against the effective UID/GID for `-O` and `-G`.
pub fn owned_by_current_token(path: &OsStr, group: bool) -> bool {
    let info = if group {
        GROUP_SECURITY_INFORMATION
    } else {
        OWNER_SECURITY_INFORMATION
    };
    let Ok(descriptor) = sys::named_security_info(path, info) else {
        return false;
    };
    let sid = if group {
        descriptor.group()
    } else {
        descriptor.owner()
    };
    sid.is_some_and(|sid| matches_token_sid(sid, group))
}

/// AccessCheck only evaluates impersonation tokens.
fn impersonation_token() -> io::Result<OwnedHandle> {
    let primary = sys::open_process_token(TOKEN_QUERY | TOKEN_DUPLICATE)?;
    sys::duplicate_token(primary.as_handle(), SecurityImpersonation)
}

/// `None` on volumes without ACLs (FAT, exFAT, many network shares), where the
/// question cannot be answered and the caller keeps the permissive answer.
fn access_check(path: &OsStr, access: u32) -> Option<bool> {
    // AccessCheck needs the owner and group SIDs next to the DACL.
    let descriptor = match sys::named_security_info(
        path,
        OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
    ) {
        Ok(descriptor) => descriptor,
        // Every file right includes READ_CONTROL, so a denied descriptor means
        // the file cannot be opened.
        Err(e) if e.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32) => return Some(false),
        Err(_) => return None,
    };
    let token = impersonation_token().ok()?;
    let mapping = GENERIC_MAPPING {
        GenericRead: FILE_GENERIC_READ,
        GenericWrite: FILE_GENERIC_WRITE,
        GenericExecute: FILE_GENERIC_EXECUTE,
        GenericAll: FILE_ALL_ACCESS,
    };
    sys::access_check(&descriptor, token.as_handle(), access, &mapping).ok()
}

fn has_access(path: &OsStr, access: u32) -> bool {
    access_check(path, access).unwrap_or(true)
}

pub fn is_readable(path: &OsStr) -> bool {
    has_access(path, FILE_GENERIC_READ)
}

/// The read-only attribute blocks writes whatever the DACL says, but NTFS
/// ignores it on directories.
pub fn is_writable(path: &OsStr, metadata: &Metadata) -> bool {
    (metadata.is_dir() || !metadata.permissions().readonly())
        && has_access(path, FILE_GENERIC_WRITE)
}

fn has_executable_extension(path: &OsStr) -> bool {
    Path::new(path)
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| {
            ["exe", "bat", "cmd", "com"]
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known))
        })
}

/// For a directory this is permission to enter it: FILE_EXECUTE doubles as
/// FILE_TRAVERSE.
pub fn is_executable(path: &OsStr, metadata: &Metadata) -> bool {
    (metadata.is_dir() || has_executable_extension(path)) && has_access(path, FILE_GENERIC_EXECUTE)
}

/// Only the standard streams can be answered for: asking the CRT about a
/// descriptor it never handed out aborts the process.
pub fn fd_is_terminal(fd: i32) -> bool {
    match fd {
        0 => io::stdin().is_terminal(),
        1 => io::stdout().is_terminal(),
        2 => io::stderr().is_terminal(),
        _ => false,
    }
}

pub fn same_file(a: &OsStr, b: &OsStr) -> bool {
    // Asking for no access right leaves nothing a share mode or DACL could
    // refuse; BACKUP_SEMANTICS lets a directory be opened.
    fn information(path: &OsStr) -> io::Result<FileInformation> {
        let file = OpenOptions::new()
            .access_mode(0)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)?;
        FileInformation::from_file(&file)
    }

    infos_refer_to_same_file(information(a), information(b))
}
