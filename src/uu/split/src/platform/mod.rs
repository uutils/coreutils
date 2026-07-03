// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(unix)]
pub use self::unix::{FilterWriter, instantiate_current_writer, paths_refer_to_same_file};

#[cfg(windows)]
pub use self::windows::{instantiate_current_writer, paths_refer_to_same_file};

#[cfg(target_os = "wasi")]
pub use self::wasi::{instantiate_current_writer, paths_refer_to_same_file};

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;

#[cfg(target_os = "wasi")]
mod wasi;

// todo: add .as_fd for std::io::copy's specialization for --bytes
pub enum Writer {
    File(std::fs::File),
    #[cfg(unix)]
    Filter(FilterWriter),
}

impl std::io::Write for Writer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::File(w) => w.write(buf),
            #[cfg(unix)]
            Self::Filter(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::File(w) => w.flush(),
            #[cfg(unix)]
            Self::Filter(w) => w.flush(),
        }
    }
}

// todo: add .as_fd for std::io::copy's specialization for --bytes
pub enum Reader {
    File(std::fs::File),
    Stdin(std::io::Stdin),
}

impl std::io::Read for Reader {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        use rustix::io::read;
        match self {
            Self::File(r) => Ok(read(r, buf)?),
            Self::Stdin(r) => Ok(read(r, buf)?),
        }
    }
}
