// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore tailable stdlib (stdlib)

use crate::args::Settings;
use crate::chunks::BytesChunkBuffer;
use crate::paths::{HeaderPrinter, PathExtTail};
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
}

impl FileHandling {
    pub fn from(settings: &Settings) -> Self {
        Self {
            map: HashMap::with_capacity(settings.inputs.len()),
            character_devices: HashSet::new(),
            last: None,
            header_printer: HeaderPrinter::new(settings.verbose, false),
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
        // A FIFO without a writer blocks `open`, and this runs on the thread that
        // serves every followed file. Open FIFOs without waiting for a writer.
        #[cfg(unix)]
        let file = crate::open_file(path, true)?;
        #[cfg(not(unix))]
        let file = File::open(path)?;
        let is_character_device = is_character_device(&file)?;
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
        #[cfg(unix)]
        let mut read_again = false;
        #[cfg(not(unix))]
        let read_again = false;
        let data = self.get_mut(path);
        if let Some(reader) = data.reader.as_mut() {
            #[cfg(unix)]
            if data.is_character_device {
                use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};

                let flags = fcntl_getfl(reader.get_ref()).map_err(io::Error::from)?;
                fcntl_setfl(reader.get_ref(), flags | OFlags::NONBLOCK).map_err(io::Error::from)?;
                // Even an always-ready device must yield to other files and --pid.
                let mut available = AvailableBytes(reader).take(64 * 1024);
                let result = chunks.fill(&mut available);
                read_again = available.limit() == 0;
                // A duplicated stdin shares these flags with the caller.
                fcntl_setfl(reader.get_ref(), flags).map_err(io::Error::from)?;
                result?;
            } else {
                chunks.fill(reader)?;
            }
            #[cfg(not(unix))]
            chunks.fill(reader)?;
        }
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
    pub fn from_other_with_path(data: Self, path: &Path) -> UResult<Self> {
        // Remove old reader
        let old_reader = data.reader;
        let reader = if old_reader.is_some() {
            // Use old reader with the same file descriptor if there is one
            old_reader
        } else if let Ok(file) = File::open(path) {
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

/// Check the descriptor rather than a path that may already name another file.
fn is_character_device(file: &File) -> io::Result<bool> {
    #[cfg(unix)]
    {
        Ok(file.metadata()?.file_type().is_char_device())
    }
    #[cfg(not(unix))]
    {
        let _ = file;
        Ok(false)
    }
}

/// End a polling pass when a nonblocking device has no more bytes available.
#[cfg(unix)]
struct AvailableBytes<'a>(&'a mut BufReader<File>);

#[cfg(unix)]
impl Read for AvailableBytes<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.0.read(buf) {
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(0),
            result => result,
        }
    }
}

#[cfg(unix)]
impl BufRead for AvailableBytes<'_> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        match self.0.fill_buf() {
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(&[]),
            result => result,
        }
    }

    fn consume(&mut self, amount: usize) {
        self.0.consume(amount);
    }
}
