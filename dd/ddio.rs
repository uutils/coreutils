use libc::types::os::arch::c95::c_int;

use std::rt::rtio;
use std::rt::rtio::{LocalIo,RtioFileStream,CloseAsynchronously,SeekCur};

use std::io::{IoResult,IoError,FileAccess,Read,Write,ReadWrite,TimedOut,ShortWrite};

pub struct RawFD {
    fd: Box<RtioFileStream>
}

impl RawFD {
    pub fn stdin() -> RawFD {
        RawFD::from_fd(0)
    }

    pub fn stdout() -> RawFD {
        RawFD::from_fd(1)
    }

    pub fn from_fd(raw_fd: i32) -> RawFD {
        match LocalIo::borrow() {
            Some(mut local_io) => {
                RawFD {
                    fd: local_io.get().fs_from_raw_fd(raw_fd as c_int, CloseAsynchronously)
                }
            },
            None => fail!("failed to get localio")
        }

    }

    pub fn open_file(path: &Path, access: FileAccess) -> IoResult<RawFD> {
        let access = match access {
            Read => rtio::Read,
            Write => rtio::Write,
            ReadWrite => rtio::ReadWrite,
        };
        LocalIo::maybe_raise(|io| {
            io.fs_open(&path.to_c_str(), rtio::Open, access).map(|fd| {
                RawFD {
                    fd: fd
                }
            })
        }).map_err(RawFD::from_rtio_error)
    }

    pub fn seek(&mut self, pos: i64) -> IoResult<u64> {
        self.fd.seek(pos, SeekCur).map_err(RawFD::from_rtio_error)
    }

    pub fn read(&mut self, buf: &mut [u8]) -> IoResult<int> {
        self.fd.read(buf).map_err(RawFD::from_rtio_error)
    }

    pub fn write(&mut self, buf: &[u8]) -> IoResult<()> {
        self.fd.write(buf).map_err(RawFD::from_rtio_error)
    }

    pub fn fsync(&mut self) -> IoResult<()> {
        self.fd.fsync().map_err(RawFD::from_rtio_error)
    }

    pub fn datasync(&mut self) -> IoResult<()> {
        self.fd.datasync().map_err(RawFD::from_rtio_error)
    }

    fn from_rtio_error(err: rtio::IoError) -> IoError {
        let rtio::IoError { code, extra, detail } = err;
        let mut ioerr = IoError::from_errno(code, false);
        ioerr.detail = detail;
        ioerr.kind = match ioerr.kind {
            TimedOut if extra > 0 => ShortWrite(extra),
            k => k,
        };
        return ioerr;
    }
}
