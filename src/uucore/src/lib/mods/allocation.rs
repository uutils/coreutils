// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Process-wide allocation failure handling for uutils binaries.

use super::locale;
use std::alloc::{GlobalAlloc, Layout, System};
use std::ptr;
use std::slice;
use std::str;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};

/// GNU-compatible allocation failure text used before localization is ready.
pub const DEFAULT_MESSAGE: &str = "memory exhausted";

/// GNU-compatible default allocation failure exit status.
pub const DEFAULT_EXIT_CODE: u8 = 1;

/// Shared Fluent message used when a utility does not provide its own OOM text.
pub const DEFAULT_MESSAGE_ID: &str = "common-memory-exhausted";

/// Per-utility allocation failure configuration.
pub struct AllocErrorConfig {
    utility: &'static str,
    exit_code: u8,
}

impl AllocErrorConfig {
    /// Create allocation failure configuration for a utility.
    pub const fn new(utility: &'static str, exit_code: u8) -> Self {
        Self { utility, exit_code }
    }

    /// Create allocation failure configuration using the GNU-compatible exit status.
    pub const fn default_for(utility: &'static str) -> Self {
        Self::new(utility, DEFAULT_EXIT_CODE)
    }
}

/// Allocation failure configuration used before the multicall binary selects a utility.
pub static COREUTILS_ALLOC_ERROR_CONFIG: AllocErrorConfig =
    AllocErrorConfig::default_for("coreutils");

static ALLOCATION_FAILURE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static ACTIVE_CONFIG: AtomicPtr<AllocErrorConfig> = AtomicPtr::new(ptr::null_mut());
static PROGRAM_PTR: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
static PROGRAM_LEN: AtomicUsize = AtomicUsize::new(0);
static MESSAGE_PTR: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
static MESSAGE_LEN: AtomicUsize = AtomicUsize::new(0);

/// Select the allocation failure policy for the utility that is about to run.
///
/// `program` and `config` must both have static storage because an allocation
/// failure can occur at any later point in the process.
pub fn activate(program: &'static str, config: &'static AllocErrorConfig) {
    // Clear a previously localized message before switching utilities. This is
    // mostly useful to embedders/tests; the normal multicall path runs one util.
    MESSAGE_PTR.store(ptr::null_mut(), Ordering::Release);
    MESSAGE_LEN.store(0, Ordering::Relaxed);

    // Publish the program before committing the config. An OOM racing this
    // update may use the previous exit code, but it will never observe the new
    // config with an unpublished program name.
    publish_text(&PROGRAM_PTR, &PROGRAM_LEN, program);
    ACTIVE_CONFIG.store(ptr::from_ref(config).cast_mut(), Ordering::Release);
}

/// Resolve the allocation failure message through the current Fluent bundle.
///
/// This must be called after localization has been initialized. Utility Fluent
/// resources are loaded with overriding semantics. A utility may customize its
/// diagnostic by defining `<util>-memory-exhausted` (for example,
/// `tsort-memory-exhausted`) in its Fluent resource. If that key is absent,
/// `common-memory-exhausted` is used. If localization is unavailable, the
/// allocator keeps using [`DEFAULT_MESSAGE`].
pub fn localize_message() {
    let config = active_config(&COREUTILS_ALLOC_ERROR_CONFIG);
    let utility = default_program(config);
    let utility_message_id = format!("{utility}-memory-exhausted");
    let utility_message = locale::get_message(&utility_message_id);

    let message = if utility_message == utility_message_id {
        locale::get_message(DEFAULT_MESSAGE_ID)
    } else {
        utility_message
    };

    if message == DEFAULT_MESSAGE_ID || message == DEFAULT_MESSAGE {
        return;
    }

    // The OOM path cannot own or allocate a String. Keep the localized value
    // alive for the remainder of the process and publish only its borrowed bytes.
    let message = Box::leak(message.into_boxed_str());
    publish_text(&MESSAGE_PTR, &MESSAGE_LEN, message);
}

fn publish_text(ptr: &AtomicPtr<u8>, len: &AtomicUsize, text: &'static str) {
    // Publish the length before the pointer. A reader that observes the pointer
    // through an Acquire load also observes the matching length.
    len.store(text.len(), Ordering::Relaxed);
    ptr.store(text.as_ptr().cast_mut(), Ordering::Release);
}

fn load_text(ptr: &AtomicPtr<u8>, len: &AtomicUsize) -> Option<&'static str> {
    let ptr = ptr.load(Ordering::Acquire);
    if ptr.is_null() {
        return None;
    }

    let len = len.load(Ordering::Relaxed);

    // SAFETY: publish_text() only publishes pointers to &'static str values,
    // and the Release/Acquire pair makes the matching length visible here.
    unsafe { Some(str::from_utf8_unchecked(slice::from_raw_parts(ptr, len))) }
}

fn active_config(default: &'static AllocErrorConfig) -> &'static AllocErrorConfig {
    let ptr = ACTIVE_CONFIG.load(Ordering::Acquire);
    if ptr.is_null() {
        default
    } else {
        // SAFETY: activate() only stores pointers obtained from &'static values.
        unsafe { &*ptr }
    }
}

fn default_program(config: &AllocErrorConfig) -> &str {
    config.utility.strip_prefix("uu_").unwrap_or(config.utility)
}

fn allocation_failed(default: &'static AllocErrorConfig) -> ! {
    let config = active_config(default);

    // Guard the best-effort diagnostic path against accidental allocator
    // re-entry on less common targets. A recursive failure must terminate
    // immediately rather than recurse indefinitely.
    if ALLOCATION_FAILURE_IN_PROGRESS.swap(true, Ordering::AcqRel) {
        raw::exit(config.exit_code);
    }
    let program = load_text(&PROGRAM_PTR, &PROGRAM_LEN).unwrap_or(default_program(config));
    let program = program.strip_prefix("uu_").unwrap_or(program);
    let message = load_text(&MESSAGE_PTR, &MESSAGE_LEN).unwrap_or(DEFAULT_MESSAGE);

    raw::write_stderr(program.as_bytes());
    raw::write_stderr(b": ");
    raw::write_stderr(message.as_bytes());
    raw::write_stderr(b"\n");
    raw::exit(config.exit_code)
}

/// Global allocator used by uutils executables.
///
/// It delegates normal allocation to [`System`]. If the system allocator
/// returns null, it emits the configured diagnostic without allocating and
/// terminates with the configured status.
pub struct UuAllocator {
    default_config: &'static AllocErrorConfig,
}

impl UuAllocator {
    /// Create an allocator with a process-startup fallback configuration.
    pub const fn new(default_config: &'static AllocErrorConfig) -> Self {
        Self { default_config }
    }

    #[cold]
    #[inline(never)]
    fn allocation_failed(&self) -> ! {
        allocation_failed(self.default_config)
    }
}

// SAFETY: every allocator operation is forwarded to System with the same
// arguments and contract. Null allocation results are converted into process
// termination before they can escape to the caller, except for zero-sized
// operations where System's result is valid and is preserved.
unsafe impl GlobalAlloc for UuAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller supplied a valid GlobalAlloc layout.
        let ptr = unsafe { System.alloc(layout) };
        if ptr.is_null() && layout.size() != 0 {
            self.allocation_failed();
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller supplied a valid GlobalAlloc layout.
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if ptr.is_null() && layout.size() != 0 {
            self.allocation_failed();
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller upholds GlobalAlloc::dealloc's contract.
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller upholds GlobalAlloc::realloc's contract.
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if new_ptr.is_null() && new_size != 0 {
            self.allocation_failed();
        }
        new_ptr
    }
}

#[cfg(unix)]
mod raw {
    use std::ffi::c_void;

    unsafe extern "C" {
        fn write(fd: i32, buf: *const c_void, count: usize) -> isize;
        fn _exit(status: i32) -> !;
    }

    pub(super) fn write_stderr(mut bytes: &[u8]) {
        while !bytes.is_empty() {
            // SAFETY: bytes is valid for bytes.len() readable bytes.
            let written = unsafe { write(2, bytes.as_ptr().cast(), bytes.len()) };
            if written <= 0 {
                return;
            }
            bytes = &bytes[written as usize..];
        }
    }

    pub(super) fn exit(code: u8) -> ! {
        // SAFETY: _exit terminates the process immediately.
        unsafe { _exit(i32::from(code)) }
    }
}

#[cfg(windows)]
mod raw {
    use std::ffi::c_void;
    use std::ptr;

    type Handle = *mut c_void;

    const STD_ERROR_HANDLE: u32 = -12_i32 as u32;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        #[link_name = "GetStdHandle"]
        fn get_std_handle(n_std_handle: u32) -> Handle;
        #[link_name = "WriteFile"]
        fn write_file(
            file: Handle,
            buffer: *const c_void,
            bytes_to_write: u32,
            bytes_written: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
        #[link_name = "ExitProcess"]
        fn exit_process(exit_code: u32) -> !;
    }

    pub(super) fn write_stderr(mut bytes: &[u8]) {
        // SAFETY: GetStdHandle accepts the documented STD_ERROR_HANDLE value.
        let handle = unsafe { get_std_handle(STD_ERROR_HANDLE) };
        if handle.is_null() {
            return;
        }

        while !bytes.is_empty() {
            let len = bytes.len().min(u32::MAX as usize);
            let mut written = 0;
            // SAFETY: buffer is valid for len bytes and written points to a u32.
            let ok = unsafe {
                write_file(
                    handle,
                    bytes.as_ptr().cast(),
                    len as u32,
                    &raw mut written,
                    ptr::null_mut(),
                )
            };
            if ok == 0 || written == 0 {
                return;
            }
            bytes = &bytes[written as usize..];
        }
    }

    pub(super) fn exit(code: u8) -> ! {
        // SAFETY: ExitProcess terminates the current process immediately.
        unsafe { exit_process(u32::from(code)) }
    }
}

#[cfg(not(any(unix, windows)))]
mod raw {
    use std::io::Write as _;

    pub(super) fn write_stderr(bytes: &[u8]) {
        let _ = std::io::stderr().write_all(bytes);
    }

    pub(super) fn exit(code: u8) -> ! {
        std::process::exit(i32::from(code))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_program_removes_utility_prefix() {
        let config = AllocErrorConfig::default_for("uu_tsort");
        assert_eq!(default_program(&config), "tsort");
    }

    #[test]
    fn published_text_is_read_after_release() {
        let ptr = AtomicPtr::new(ptr::null_mut());
        let len = AtomicUsize::new(0);
        publish_text(&ptr, &len, "memory exhausted");
        assert_eq!(load_text(&ptr, &len), Some("memory exhausted"));
    }
}
