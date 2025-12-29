// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore reflink
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use uucore::buf_copy;
use uucore::display::Quotable;
use uucore::translate;

use uucore::mode::get_umask;

use crate::{
    CopyDebug, CopyResult, CpError, OffloadReflinkDebug, ReflinkMode, SparseDebug, SparseMode,
    is_stream,
};

/// Copies `source` to `dest` using copy-on-write if possible.
pub(crate) fn copy_on_write(
    source: &Path,
    dest: &Path,
    reflink_mode: ReflinkMode,
    sparse_mode: SparseMode,
    context: &str,
    source_is_stream: bool,
) -> CopyResult<CopyDebug> {
    if sparse_mode != SparseMode::Auto {
        return Err(translate!("cp-error-sparse-not-supported")
            .to_string()
            .into());
    }
    let mut copy_debug = CopyDebug {
        offload: OffloadReflinkDebug::Unknown,
        reflink: OffloadReflinkDebug::Unsupported,
        sparse_detection: SparseDebug::Unsupported,
    };

    // Extract paths in a form suitable to be passed to a syscall.
    // The unwrap() is safe because they come from the command-line and so contain non nul
    // character.
    let src = CString::new(source.as_os_str().as_bytes()).unwrap();
    let dst = CString::new(dest.as_os_str().as_bytes()).unwrap();

    // clonefile(2) was introduced in macOS 10.12 so we cannot statically link against it
    // for backward compatibility.
    let clonefile = CString::new("clonefile").unwrap();
    let raw_pfn = unsafe { libc::dlsym(libc::RTLD_NEXT, clonefile.as_ptr()) };

    let mut error = 0;
    if !raw_pfn.is_null() {
        // Call clonefile(2).
        // Safety: Casting a C function pointer to a rust function value is one of the few
        // blessed uses of `transmute()`.
        unsafe {
            let pfn: extern "C" fn(
                src: *const libc::c_char,
                dst: *const libc::c_char,
                flags: u32,
            ) -> libc::c_int = std::mem::transmute(raw_pfn);
            error = pfn(src.as_ptr(), dst.as_ptr(), 0);
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::AlreadyExists
                // Only remove the `dest` if the `source` and `dest` are not the same
                && source != dest
            {
                // clonefile(2) fails if the destination exists.  Remove it and try again.  Do not
                // bother to check if removal worked because we're going to try to clone again.
                // first lets make sure the dest file is not read only
                if fs::metadata(dest).is_ok_and(|md| !md.permissions().readonly()) {
                    // remove and copy again
                    // TODO: rewrite this to better match linux behavior
                    // linux first opens the source file and destination file then uses the file
                    // descriptors to do the clone.
                    let _ = fs::remove_file(dest);
                    error = pfn(src.as_ptr(), dst.as_ptr(), 0);
                }
            }
        }
    }

    if raw_pfn.is_null() || error != 0 {
        // clonefile(2) is either not supported or it errored out (possibly because the FS does not
        // support COW).
        match reflink_mode {
            ReflinkMode::Always => {
                return Err(translate!("cp-error-failed-to-clone", "source" => source.quote(), "dest" => dest.quote(), "error" => error)
                .into());
            }
            _ => {
                copy_debug.reflink = OffloadReflinkDebug::Yes;
                if source_is_stream {
                    let mut src_file = File::open(source)?;
                    let mode = 0o622 & !get_umask();
                    let mut dst_file = OpenOptions::new()
                        .create(true)
                        .write(true)
                        .mode(mode)
                        .open(dest)?;

                    let dest_is_stream = is_stream(&dst_file.metadata()?);
                    if !dest_is_stream {
                        // `copy_stream` doesn't clear the dest file, if dest is not a stream, we should clear it manually.
                        dst_file.set_len(0)?;
                    }

                    buf_copy::copy_stream(&mut src_file, &mut dst_file)
                        .map_err(|_| std::io::Error::from(std::io::ErrorKind::Other))
                        .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?
                } else {
                    fs::copy(source, dest)
                        .map_err(|e| CpError::IoErrContext(e, context.to_owned()))?
                }
            }
        };
    }

    Ok(copy_debug)
}
