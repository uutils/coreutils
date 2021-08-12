use std::ffi::{CStr, CString, OsStr};
use std::marker::PhantomData;
use std::os::raw::{c_int, c_long, c_short};
use std::path::Path;
use std::ptr::NonNull;
use std::{io, iter, ptr, slice};

use crate::errors::{Error, Result};
use crate::os_str_to_c_string;

#[derive(Debug)]
pub(crate) struct FTS {
    fts: ptr::NonNull<fts_sys::FTS>,

    entry: Option<ptr::NonNull<fts_sys::FTSENT>>,
    _phantom_data: PhantomData<fts_sys::FTSENT>,
}

impl FTS {
    pub(crate) fn new<I>(paths: I, options: c_int) -> Result<Self>
    where
        I: IntoIterator,
        I::Item: AsRef<OsStr>,
    {
        let files_paths: Vec<CString> = paths
            .into_iter()
            .map(|s| os_str_to_c_string(s.as_ref()))
            .collect::<Result<_>>()?;

        if files_paths.is_empty() {
            return Err(Error::from_io(
                "FTS::new()",
                io::ErrorKind::InvalidInput.into(),
            ));
        }

        let path_argv: Vec<_> = files_paths
            .iter()
            .map(CString::as_ref)
            .map(CStr::as_ptr)
            .chain(iter::once(ptr::null()))
            .collect();

        // SAFETY: We assume calling fts_open() is safe:
        // - `path_argv` is an array holding at least one path, and null-terminated.
        // - `compar` is None.
        let fts = unsafe { fts_sys::fts_open(path_argv.as_ptr().cast(), options, None) };

        let fts = ptr::NonNull::new(fts)
            .ok_or_else(|| Error::from_io("fts_open()", io::Error::last_os_error()))?;

        Ok(Self {
            fts,
            entry: None,
            _phantom_data: PhantomData,
        })
    }

    pub(crate) fn last_entry_ref(&mut self) -> Option<EntryRef> {
        self.entry.map(move |entry| EntryRef::new(self, entry))
    }

    pub(crate) fn read_next_entry(&mut self) -> Result<bool> {
        // SAFETY: We assume calling fts_read() is safe with a non-null `fts`
        // pointer assumed to be valid.
        let new_entry = unsafe { fts_sys::fts_read(self.fts.as_ptr()) };

        self.entry = NonNull::new(new_entry);
        if self.entry.is_none() {
            let r = io::Error::last_os_error();
            if let Some(0) = r.raw_os_error() {
                Ok(false)
            } else {
                Err(Error::from_io("fts_read()", r))
            }
        } else {
            Ok(true)
        }
    }

    pub(crate) fn set(&mut self, instr: c_int) -> Result<()> {
        let fts = self.fts.as_ptr();
        let entry = self
            .entry
            .ok_or_else(|| Error::from_io("FTS::set()", io::ErrorKind::UnexpectedEof.into()))?;

        // SAFETY: We assume calling fts_set() is safe with non-null `fts`
        // and `entry` pointers assumed to be valid.
        if unsafe { fts_sys::fts_set(fts, entry.as_ptr(), instr) } == -1 {
            Err(Error::from_io("fts_set()", io::Error::last_os_error()))
        } else {
            Ok(())
        }
    }
}

impl Drop for FTS {
    fn drop(&mut self) {
        // SAFETY: We assume calling fts_close() is safe with a non-null `fts`
        // pointer assumed to be valid.
        unsafe { fts_sys::fts_close(self.fts.as_ptr()) };
    }
}

#[derive(Debug)]
pub(crate) struct EntryRef<'fts> {
    pub(crate) pointer: ptr::NonNull<fts_sys::FTSENT>,

    _fts: PhantomData<&'fts FTS>,
    _phantom_data: PhantomData<fts_sys::FTSENT>,
}

impl<'fts> EntryRef<'fts> {
    fn new(_fts: &'fts FTS, entry: ptr::NonNull<fts_sys::FTSENT>) -> Self {
        Self {
            pointer: entry,
            _fts: PhantomData,
            _phantom_data: PhantomData,
        }
    }

    fn as_ref(&self) -> &fts_sys::FTSENT {
        // SAFETY: `self.pointer` is a non-null pointer that is assumed to be valid.
        unsafe { self.pointer.as_ref() }
    }

    fn as_mut(&mut self) -> &mut fts_sys::FTSENT {
        // SAFETY: `self.pointer` is a non-null pointer that is assumed to be valid.
        unsafe { self.pointer.as_mut() }
    }

    pub(crate) fn flags(&self) -> c_int {
        c_int::from(self.as_ref().fts_info)
    }

    pub(crate) fn errno(&self) -> c_int {
        self.as_ref().fts_errno
    }

    pub(crate) fn level(&self) -> c_short {
        self.as_ref().fts_level
    }

    pub(crate) fn number(&self) -> c_long {
        self.as_ref().fts_number
    }

    pub(crate) fn set_number(&mut self, new_number: c_long) {
        self.as_mut().fts_number = new_number;
    }

    pub(crate) fn path(&self) -> Option<&Path> {
        let entry = self.as_ref();
        if entry.fts_pathlen == 0 {
            return None;
        }

        NonNull::new(entry.fts_path)
            .map(|path_ptr| {
                let path_size = usize::from(entry.fts_pathlen).saturating_add(1);

                // SAFETY: `entry.fts_path` is a non-null pointer that is assumed to be valid.
                unsafe { slice::from_raw_parts(path_ptr.as_ptr().cast(), path_size) }
            })
            .and_then(|bytes| CStr::from_bytes_with_nul(bytes).ok())
            .map(c_str_to_os_str)
            .map(Path::new)
    }

    pub(crate) fn access_path(&self) -> Option<&Path> {
        ptr::NonNull::new(self.as_ref().fts_accpath)
            .map(|path_ptr| {
                // SAFETY: `entry.fts_accpath` is a non-null pointer that is assumed to be valid.
                unsafe { CStr::from_ptr(path_ptr.as_ptr()) }
            })
            .map(c_str_to_os_str)
            .map(Path::new)
    }

    pub(crate) fn stat(&self) -> Option<&libc::stat> {
        ptr::NonNull::new(self.as_ref().fts_statp).map(|stat_ptr| {
            // SAFETY: `entry.fts_statp` is a non-null pointer that is assumed to be valid.
            unsafe { stat_ptr.as_ref() }
        })
    }
}

#[cfg(unix)]
fn c_str_to_os_str(s: &CStr) -> &OsStr {
    use std::os::unix::ffi::OsStrExt;

    OsStr::from_bytes(s.to_bytes())
}
