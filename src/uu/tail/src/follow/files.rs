// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore tailable stdlib (stdlib)
// spell-checker:ignore nonblocking POLLNVAL

use crate::args::Settings;
use crate::chunks::BytesChunkBuffer;
use crate::open_file;
use crate::paths::{HeaderPrinter, PathExtTail};
use crate::read::ReadAvailable;
use crate::text;
use std::collections::hash_map::Keys;
use std::collections::{HashMap, HashSet};
use std::fs::{File, Metadata};
use std::io::{self, BufReader, BufWriter, Write, stdout};
#[cfg(unix)]
use std::io::{BufRead, Read};
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use uucore::error::UResult;

/// Data structure to keep a handle on files to follow.
/// `last` always holds the path/key of the last file that was printed from.
/// The keys of the [`HashMap`] can point to an existing file path (normal case),
/// or stdin ("-"), or to a non-existing path (--retry).
/// For existing files, all keys in the [`HashMap`] are absolute Paths.
pub struct FileHandling {
    map: HashMap<PathBuf, PathData>,
    character_devices: HashSet<PathBuf>,
    last: Option<PathBuf>,
    header_printer: HeaderPrinter,
    pub(super) nonblocking: bool,
}

impl FileHandling {
    pub fn from(settings: &Settings) -> Self {
        Self {
            map: HashMap::with_capacity(settings.inputs.len()),
            character_devices: HashSet::new(),
            last: None,
            header_printer: HeaderPrinter::new(settings.verbose, false),
            nonblocking: settings.use_nonblocking(),
        }
    }

    /// Wrapper for [`HashMap::insert`] using [`Path::canonicalize`]
    pub fn insert(&mut self, k: &Path, v: PathData, update_last: bool) {
        let k = Self::canonicalize_path(k);
        if update_last {
            self.last = Some(k.clone());
        }
        if v.is_character_device {
            self.character_devices.insert(k.clone());
        } else {
            self.character_devices.remove(&k);
        }
        let _ = self.map.insert(k, v);
    }

    /// Wrapper for [`HashMap::remove`] using [`Path::canonicalize`]
    pub fn remove(&mut self, k: &Path) -> PathData {
        let k = Self::canonicalize_path(k);
        self.character_devices.remove(&k);
        self.map.remove(&k).unwrap()
    }

    /// Wrapper for [`HashMap::get`] using [`Path::canonicalize`]
    pub fn get(&self, k: &Path) -> &PathData {
        self.map.get(&Self::canonicalize_path(k)).unwrap()
    }

    /// Wrapper for [`HashMap::get_mut`] using [`Path::canonicalize`]
    pub fn get_mut(&mut self, k: &Path) -> &mut PathData {
        self.map.get_mut(&Self::canonicalize_path(k)).unwrap()
    }

    /// Canonicalize `path` if it is not already an absolute path
    fn canonicalize_path(path: &Path) -> PathBuf {
        if path.is_relative()
            && !path.is_stdin()
            && let Ok(p) = path.canonicalize()
        {
            return p;
        }
        path.to_owned()
    }

    pub fn get_mut_metadata(&mut self, path: &Path) -> Option<&Metadata> {
        self.get_mut(path).metadata.as_ref()
    }

    pub fn keys(&self) -> Keys<'_, PathBuf, PathData> {
        self.map.keys()
    }

    pub fn character_devices(&self) -> &HashSet<PathBuf> {
        &self.character_devices
    }

    pub fn contains_key(&self, k: &Path) -> bool {
        self.map.contains_key(k)
    }

    pub fn get_last(&self) -> Option<&PathBuf> {
        self.last.as_ref()
    }

    /// Return true if there is only stdin remaining
    pub fn only_stdin_remaining(&self) -> bool {
        self.map.len() == 1 && (self.map.contains_key(Path::new(text::DASH)))
    }

    /// Return true if there is at least one "tailable" path (or stdin) remaining
    pub fn files_remaining(&self) -> bool {
        for path in self.map.keys() {
            if path.is_tailable() || path.is_stdin() {
                return true;
            }
        }
        false
    }

    /// Returns true if there are no files remaining
    pub fn no_files_remaining(&self, settings: &Settings) -> bool {
        self.map.is_empty() || !self.files_remaining() && !settings.retry
    }

    /// Set `reader` to None to indicate that `path` is not an existing file anymore.
    pub fn reset_reader(&mut self, path: &Path) {
        let key = Self::canonicalize_path(path);
        let data = self.map.get_mut(&key).unwrap();
        data.reader = None;
        data.is_character_device = false;
        self.character_devices.remove(&key);
    }

    /// Reopen the file at the monitored `path`
    pub fn update_reader(&mut self, path: &Path) -> UResult<()> {
        let (file, is_character_device) = open_follow_file(path, self.nonblocking)?;
        let key = Self::canonicalize_path(path);
        let data = self.map.get_mut(&key).unwrap();
        data.reader.replace(BufReader::new(file));
        data.is_character_device = is_character_device;
        data.symlink = path.is_symlink();
        if is_character_device {
            self.character_devices.insert(key);
        } else {
            self.character_devices.remove(&key);
        }
        Ok(())
    }

    /// Reload metadata from `path`, or `metadata`
    pub fn update_metadata(&mut self, path: &Path, metadata: Option<Metadata>) {
        self.get_mut(path).metadata = if metadata.is_some() {
            metadata
        } else {
            path.metadata().ok()
        };
    }

    /// Read new data from `path` and print it to stdout.
    /// Return true when a device filled its batch and should be read again without waiting.
    pub fn tail_file(&mut self, path: &Path, verbose: bool) -> UResult<bool> {
        let mut chunks = BytesChunkBuffer::new(u64::MAX);
        let read_again = self.get_mut(path).read_available(&mut chunks)?;
        if chunks.has_data() {
            if self.needs_header(path, verbose) {
                let display_name = self.get(path).display_name.clone();
                self.header_printer.print(display_name.as_str())?;
            }

            let mut writer = BufWriter::new(stdout().lock());
            chunks.write(&mut writer)?;
            writer.flush()?;

            self.last.replace(path.to_owned());
            self.update_metadata(path, None);
        }
        Ok(read_again)
    }

    /// Decide if printing `path` needs a header based on when it was last printed
    pub fn needs_header(&self, path: &Path, verbose: bool) -> bool {
        if verbose {
            if let Some(ref last) = self.last {
                !last.eq(&path)
            } else {
                true
            }
        } else {
            false
        }
    }
}

/// Data structure to keep a handle on the [`BufReader`], [`Metadata`]
/// and the `display_name` (`header_name`) of files that are being followed.
pub struct PathData {
    pub reader: Option<BufReader<File>>,
    // Describes the open descriptor, which can outlive the file at its path.
    is_character_device: bool,
    pub metadata: Option<Metadata>,
    pub display_name: String,
    /// Whether the path was a symbolic link when it was last opened. Under
    /// `--follow=name`, a link named on the command line is followed to
    /// whatever it points at, while a regular file that turns into a link is
    /// not: its target could be any file.
    pub symlink: bool,
}

impl PathData {
    /// Fill one batch, returning whether a device may have more data ready.
    fn read_available(&mut self, chunks: &mut BytesChunkBuffer) -> UResult<bool> {
        let Some(reader) = self.reader.as_mut() else {
            return Ok(false);
        };
        #[cfg(unix)]
        if self.is_character_device {
            return read_device(reader, chunks);
        }
        chunks.fill(&mut ReadAvailable::new(reader))?;
        Ok(false)
    }

    pub fn new(
        reader: Option<BufReader<File>>,
        metadata: Option<Metadata>,
        display_name: &str,
        symlink: bool,
    ) -> UResult<Self> {
        let is_character_device = reader
            .as_ref()
            .map(|reader| is_character_device(reader.get_ref()))
            .transpose()?
            .unwrap_or(false);
        Ok(Self {
            reader,
            is_character_device,
            metadata,
            display_name: display_name.to_owned(),
            symlink,
        })
    }
    pub fn from_other_with_path(data: Self, path: &Path, nonblocking: bool) -> UResult<Self> {
        // Remove old reader
        let old_reader = data.reader;
        let reader = if old_reader.is_some() {
            // Use old reader with the same file descriptor if there is one
            old_reader
        } else if let Ok((file, _)) = open_follow_file(path, nonblocking) {
            // Open new file tail from start
            Some(BufReader::new(file))
        } else {
            // Probably file was renamed/moved or removed again
            None
        };

        Self::new(
            reader,
            path.metadata().ok(),
            data.display_name.as_str(),
            path.is_symlink(),
        )
    }
}

fn open_follow_file(
    path: &Path,
    #[cfg(unix)] nonblocking: bool,
    #[cfg(not(unix))] _: bool,
) -> io::Result<(File, bool)> {
    // A FIFO without a writer must not block the thread serving all files.
    let file = open_file(path, true)?;
    let is_character_device = is_character_device(&file)?;
    #[cfg(unix)]
    if !nonblocking && !is_character_device {
        use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};

        // Devices must stay nonblocking even if another reader consumes their
        // input between the readiness check and the read. Preserve the existing
        // blocking behavior for other files when --pid is absent.
        let flags = fcntl_getfl(&file).map_err(io::Error::from)?;
        fcntl_setfl(&file, flags & !OFlags::NONBLOCK).map_err(io::Error::from)?;
    }
    Ok((file, is_character_device))
}

/// Check the descriptor rather than a path that may already name another file.
#[cfg(unix)]
fn is_character_device(file: &File) -> io::Result<bool> {
    Ok(file.metadata()?.file_type().is_char_device())
}

#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)] // Match the fallible Unix implementation.
fn is_character_device(_file: &File) -> io::Result<bool> {
    Ok(false)
}

#[cfg(unix)]
fn read_device(reader: &mut BufReader<File>, chunks: &mut BytesChunkBuffer) -> UResult<bool> {
    // Even an always-ready device must yield to other files and --pid.
    let mut available = ReadAvailable::new(AvailableBytes(reader)).take(64 * 1024);
    chunks.fill(&mut available)?;
    Ok(available.limit() == 0)
}

#[cfg(all(unix, not(target_vendor = "apple")))]
fn device_is_readable(file: &File) -> io::Result<bool> {
    use rustix::event::{PollFd, PollFlags, Timespec, poll};

    let mut fds = [PollFd::new(file, PollFlags::IN)];
    rustix::io::retry_on_intr(|| poll(&mut fds, Some(&Timespec::default())))
        .map(|ready| ready != 0)
        .map_err(io::Error::from)
}

#[cfg(target_vendor = "apple")]
fn device_is_readable(file: &File) -> io::Result<bool> {
    use rustix::event::{FdSetElement, Timespec, fd_set_insert, fd_set_num_elements, select};
    use std::os::fd::AsRawFd;

    // Apple's poll reports POLLNVAL for devices such as /dev/null and /dev/zero.
    let fd = file.as_raw_fd();
    let mut read_fds = vec![FdSetElement::default(); fd_set_num_elements(1, fd + 1)];
    rustix::io::retry_on_intr(|| {
        fd_set_insert(&mut read_fds, fd);
        // SAFETY: file keeps the sole descriptor in the correctly sized set open.
        unsafe {
            select(
                fd + 1,
                Some(&mut read_fds),
                None,
                None,
                Some(&Timespec::default()),
            )
        }
    })
    .map(|ready| ready != 0)
    .map_err(io::Error::from)
}

/// Check readiness before each refill without changing flags shared with stdin's owner.
/// Owned devices are nonblocking; inherited stdin can still race with another reader.
#[cfg(unix)]
struct AvailableBytes<'a>(&'a mut BufReader<File>);

#[cfg(unix)]
impl Read for AvailableBytes<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = self.fill_buf()?.read(buf)?;
        self.consume(count);
        Ok(count)
    }
}

#[cfg(unix)]
impl BufRead for AvailableBytes<'_> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.0.buffer().is_empty() && !device_is_readable(self.0.get_ref())? {
            return Ok(&[]);
        }
        self.0.fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        self.0.consume(amount);
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use rustix::fs::{OFlags, fcntl_getfl};
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;
    use std::sync::Barrier;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn reopened_device_stays_nonblocking() {
        let settings = Settings::default();
        let path = Path::new("/dev/null");
        let mut files = FileHandling::from(&settings);
        files.insert(
            path,
            PathData::new(None, None, "device", false).unwrap(),
            false,
        );

        files.update_reader(path).unwrap();

        let reader = files.get(path).reader.as_ref().unwrap();
        assert!(
            fcntl_getfl(reader.get_ref())
                .unwrap()
                .contains(OFlags::NONBLOCK)
        );
    }

    #[test]
    fn device_reads_preserve_shared_flags() {
        let file = File::open("/dev/zero").unwrap();
        let shared = file.try_clone().unwrap();
        let flags = fcntl_getfl(&shared).unwrap();
        assert!(!flags.contains(OFlags::NONBLOCK));
        let mut reader = BufReader::new(file);
        let done = AtomicBool::new(false);
        let started = Barrier::new(2);

        std::thread::scope(|scope| {
            let monitor = scope.spawn(|| {
                started.wait();
                while !done.load(Ordering::Relaxed) {
                    if fcntl_getfl(&shared).unwrap() != flags {
                        return false;
                    }
                }
                true
            });
            started.wait();
            // Observe the shared description during reads, not just after flags
            // could have been restored. Each batch remains bounded at 64 KiB.
            let result: UResult<()> = (0..128).try_for_each(|_| {
                read_device(&mut reader, &mut BytesChunkBuffer::new(u64::MAX))?;
                Ok(())
            });
            done.store(true, Ordering::Relaxed);
            let unchanged = monitor.join().unwrap();
            result.unwrap();
            assert!(unchanged, "device reads changed shared file status flags");
        });
    }

    #[test]
    fn available_bytes_preserve_buffered_data() {
        let (input, mut output) = UnixStream::pair().unwrap();
        let mut reader = BufReader::new(File::from(OwnedFd::from(input)));
        output.write_all(b"data").unwrap();
        assert_eq!(reader.fill_buf().unwrap(), b"data");
        reader.consume(1);

        // The descriptor is now quiet, but three bytes remain in the buffer.
        let mut bytes = Vec::new();
        AvailableBytes(&mut reader).read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"ata");
        assert!(reader.buffer().is_empty());
    }

    #[test]
    fn device_hangup_drains_data_without_busy_looping() {
        let (input, mut output) = UnixStream::pair().unwrap();
        let mut reader = BufReader::new(File::from(OwnedFd::from(input)));
        output.write_all(b"data").unwrap();
        drop(output);

        let mut chunks = BytesChunkBuffer::new(u64::MAX);
        assert!(!read_device(&mut reader, &mut chunks).unwrap());
        let mut bytes = Vec::new();
        chunks.write(&mut bytes).unwrap();
        assert_eq!(bytes, b"data");

        let mut chunks = BytesChunkBuffer::new(u64::MAX);
        assert!(!read_device(&mut reader, &mut chunks).unwrap());
        assert!(!chunks.has_data());
    }
}
