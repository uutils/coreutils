use std::io::{self, Write};
use std::os::unix::io::RawFd;

use libc::{O_APPEND, S_IFIFO, S_IFREG};
use nix::errno::Errno;
use nix::fcntl::{fcntl, splice, vmsplice, FcntlArg, SpliceFFlags};
use nix::sys::stat::{fstat, FileStat};
use nix::sys::uio::IoVec;
use nix::unistd::pipe;
use platform_info::{PlatformInfo, Uname};

use crate::features::zero_copy::{FromRawObject, RawObject};

lazy_static::lazy_static! {
    static ref IN_WSL: bool = {
        let info = PlatformInfo::new().unwrap();
        info.release().contains("Microsoft")
    };
}

pub struct PlatformZeroCopyWriter {
    raw_obj: RawObject,
    read_pipe: RawFd,
    write_pipe: RawFd,
    write_fn: fn(&mut PlatformZeroCopyWriter, &[IoVec<&[u8]>], usize) -> io::Result<usize>,
}

impl PlatformZeroCopyWriter {
    pub unsafe fn new(raw_obj: RawObject) -> nix::Result<Self> {
        if *IN_WSL {
            // apparently WSL hasn't implemented vmsplice(), causing writes to fail
            // thus, we will just say zero-copy doesn't work there rather than working
            // around it
            return Err(nix::Error::from(Errno::EOPNOTSUPP));
        }

        let stat_info: FileStat = fstat(raw_obj)?;
        let access_mode: libc::c_int = fcntl(raw_obj, FcntlArg::F_GETFL)?;

        let is_regular = (stat_info.st_mode & S_IFREG) != 0;
        let is_append = (access_mode & O_APPEND) != 0;
        let is_fifo = (stat_info.st_mode & S_IFIFO) != 0;

        if is_regular && !is_append {
            let (read_pipe, write_pipe) = pipe()?;

            Ok(PlatformZeroCopyWriter {
                raw_obj,
                read_pipe,
                write_pipe,
                write_fn: write_regular,
            })
        } else if is_fifo {
            Ok(PlatformZeroCopyWriter {
                raw_obj,
                read_pipe: Default::default(),
                write_pipe: Default::default(),
                write_fn: write_fifo,
            })
        } else {
            // FIXME: how to error?
            Err(nix::Error::from(Errno::UnknownErrno))
        }
    }
}

impl FromRawObject for PlatformZeroCopyWriter {
    unsafe fn from_raw_object(obj: RawObject) -> Option<Self> {
        PlatformZeroCopyWriter::new(obj).ok()
    }
}

impl Write for PlatformZeroCopyWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let iovec = &[IoVec::from_slice(buf)];

        let func = self.write_fn;
        func(self, iovec, buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // XXX: not sure if we need anything else
        Ok(())
    }
}

fn write_regular(
    writer: &mut PlatformZeroCopyWriter,
    iovec: &[IoVec<&[u8]>],
    len: usize,
) -> io::Result<usize> {
    vmsplice(writer.write_pipe, iovec, SpliceFFlags::empty())
        .and_then(|_| {
            splice(
                writer.read_pipe,
                None,
                writer.raw_obj,
                None,
                len,
                SpliceFFlags::empty(),
            )
        })
        .map_err(|_| io::Error::last_os_error())
}

fn write_fifo(
    writer: &mut PlatformZeroCopyWriter,
    iovec: &[IoVec<&[u8]>],
    _len: usize,
) -> io::Result<usize> {
    vmsplice(writer.raw_obj, iovec, SpliceFFlags::empty()).map_err(|_| io::Error::last_os_error())
}
