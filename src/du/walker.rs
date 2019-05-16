use crate::{Options, TimeFormat, TimeStrategy};

use crossbeam::channel;
use time::Timespec;

use std::collections::{HashSet, HashMap};
use std::collections::hash_map::Entry as HashMapEntry;
use std::fs::{self, DirEntry, Metadata};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::os::unix::fs::MetadataExt;
use std::sync::{Arc, Mutex};

// XXX: this has not been tuned much at all
const MAX_MESSAGES: usize = 1024 * 64;
const MAX_DEFAULT_THREADS: usize = 8;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct FileID {
    device: u64,
    inode: u64,
}

impl From<&Metadata> for FileID {
    fn from(metadata: &Metadata) -> FileID {
        FileID {
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Stat {
    size: u64,
    blocks: u64,
    /// Either time of last status change, access, or modification for file
    time: u64,
}

impl Stat {
    fn new(metadata: &Metadata, options: &Options) -> Stat {
        // FIXME: this doesn't work quite right because "--time" is not handled
        let time = match options.time {
            Some(strat) => match strat {
                // time of last status change for file
                TimeStrategy::Ctime | TimeStrategy::Status | TimeStrategy::Use => metadata.ctime(),
                // time of last access for file
                TimeStrategy::Atime | TimeStrategy::Access => metadata.atime(),
            },
            // time of last modification for file
            None => metadata.mtime()
        } as u64;

        Stat {
            size: metadata.len(),
            blocks: metadata.blocks() as u64,
            time,
        }
    }
}

// NOTE: this is non-commutative because "time" will be the same as that of self
impl Add for Stat {
    type Output = Stat;

    fn add(self, other: Stat) -> Stat {
        Stat {
            size: self.size + other.size,
            blocks: self.blocks + other.blocks,
            time: self.time,
        }
    }
}

// NOTE: same as for Add
impl AddAssign for Stat {
    fn add_assign(&mut self, other: Stat) {
        *self = *self + other;
    }
}

// NOTE: same as for Add
impl Sub for Stat {
    type Output = Stat;

    fn sub(self, other: Stat) -> Stat {
        Stat {
            size: self.size - other.size,
            blocks: self.blocks - other.blocks,
            time: self.time,
        }
    }
}

// NOTE: same as for Add
impl SubAssign for Stat {
    fn sub_assign(&mut self, other: Stat) {
        *self = *self - other;
    }
}

// XXX: should try to reduce the size of the messages
enum IoMessage {
    RegisterParent {
        dir_id: FileID,
        parent_id: Option<FileID>,
        path: PathBuf,
        stat: Stat,
        subdir_count: usize,
        depth: usize,
    },
    FinishedFiles {
        files: Vec<(PathBuf, Stat)>,
    },
    FinishedDir {
        path: PathBuf,
        stat: Stat,
        parent_id: Option<FileID>,
        depth: usize,
    },
    CouldNotReadDir {
        path: PathBuf,
        stat: Stat,
        parent_id: Option<FileID>,
        depth: usize,
        err: io::Error,
    },
    FailedToGetMetadata {
        path: PathBuf,
        err: io::Error,
    },
    InvalidEntry {
        err: io::Error,
    },
}

enum TraversalMessage {
    Subdir {
        path: PathBuf,
        metadata: Metadata,
        parent_id: Option<FileID>,
        depth: usize,
    },
    Done,
}

pub(crate) struct DuWalker<'a, F: Send + Sync + Fn(u64) -> String> {
    path: PathBuf,
    metadata: Metadata,
    convert_size: F,
    options: &'a Options,
    thread_count: usize,
}

// XXX: it would be great to keep the threads running after run() (and take &self or something
//      instead of self) so we don't have to spin them up again when we begin trawling through
//      the directories
impl<'a, F: Send + Sync + Fn(u64) -> String> DuWalker<'a, F> {
    pub fn new(path: PathBuf, options: &'a Options, convert_size: F) -> io::Result<Self> {
        // XXX: maybe this should be given a path and metadata?  only work for directories?
        let metadata = fs::symlink_metadata(&path)?;

        let mut thread_count = options.thread_count;
        if thread_count == 0 {
            thread_count = num_cpus::get().min(MAX_DEFAULT_THREADS);
        }
        if thread_count > 1 {
            // the current thread should already be taking up one of the threads
            thread_count -= 1;
        }

        Ok(Self {
            path,
            metadata,
            options,
            convert_size,
            thread_count,
        })
    }

    pub fn run(self) -> Option<u64> {
        // XXX: should probably test if unbounded or bounded work better
        let (io_tx, io_rx) = channel::bounded(MAX_MESSAGES);
        let (subdir_tx, subdir_rx) = channel::unbounded();

        let path = self.path;
        let options = self.options;
        let metadata = self.metadata;
        let convert_size = self.convert_size;
        let thread_count = self.thread_count;

        let ids = Arc::new(Mutex::new(HashSet::new()));

        // TODO: check if the given file is NOT a directory, if so, either just print the data for
        //       the file by spawning the IoWorker and NOT spawning the TraversalWorkers, or just
        //       print directly

        crossbeam::scope(|s| {
            // FIXME: the number of threads should perhaps be less than the number of cpus?  i/o bound
            for _ in 0..thread_count {
                s.spawn(|_| {
                    let worker = TraversalWorker::new(options, ids.clone(), &subdir_rx, &subdir_tx, &io_tx);
                    worker.walk();

                    // if we get here we need to send another "Done" message to ensure all the
                    // threads exit
                    subdir_tx.send(TraversalMessage::Done);
                });
            }

            subdir_tx.send(TraversalMessage::Subdir {
                path: path.clone(),
                metadata,
                parent_id: None,
                depth: 0,
            });

            // XXX: not sure but it might be best to spawn several IoWorkers and synchronize them
            //      somehow (or just limit the number of TraversalWorkers to prevent them from just
            //      flooding the IoWorker with data)
            let io_worker = IoWorker::new(path, io_rx, options, convert_size);
            let maybe_stat = io_worker.run();

            subdir_tx.send(TraversalMessage::Done);

            maybe_stat.map(|stat| stat.size)
        }).unwrap()
    }
}

struct TraversalWorker<'a> {
    subdir_rx: &'a channel::Receiver<TraversalMessage>,
    subdir_tx: &'a channel::Sender<TraversalMessage>,
    io_tx: &'a channel::Sender<IoMessage>,

    options: &'a Options,
    ids: Arc<Mutex<HashSet<FileID>>>,
}

// FIXME: i believe this overflows due to the large amount of data stored and the continuous
//        recursion.  one way around this might be to create another set of channels so we
//        can just push subdirectories into the write channel to be processed by another thread
//        waiting for data from a read channel.  unsure how to handle ids and the FinishedDir
//        message in this case, however
impl<'a> TraversalWorker<'a> {
    pub fn new(options: &'a Options, ids: Arc<Mutex<HashSet<FileID>>>, subdir_rx: &'a channel::Receiver<TraversalMessage>, subdir_tx: &'a channel::Sender<TraversalMessage>, io_tx: &'a channel::Sender<IoMessage>) -> Self {
        Self {
            subdir_rx,
            subdir_tx,
            io_tx,

            options,
            ids,
        }
    }

    pub fn walk(mut self) {
        for msg in self.subdir_rx {
            match msg {
                TraversalMessage::Subdir { path, metadata, parent_id, depth } => {            
                    if let Err(msg) = self.walk_helper(path, metadata, parent_id, depth) {
                        self.io_tx.send(msg);
                    }
                }
                TraversalMessage::Done => break,
            }
        }
    }

    fn walk_helper(&mut self, path: PathBuf, metadata: Metadata, parent_id: Option<FileID>, depth: usize) -> Result<(), IoMessage> {
        let mut stat = Stat::new(&metadata, self.options);

        // FIXME: avoid this clone
        let dir_iter = fs::read_dir(&path)
            .map_err(|e| IoMessage::CouldNotReadDir {
                path: path.clone(),
                stat,
                parent_id,
                depth,
                err: e
            })?;

        let mut dirs = vec![];
        let mut possible_dups = vec![];
        let mut files = if self.options.all {
            vec![]
        } else {
            Vec::with_capacity(0)
        };

        for f in dir_iter {
            match self.handle_entry(f, &mut dirs, &mut possible_dups, &mut files) {
                Ok(Some(file_stat)) => {
                    stat += file_stat;
                }
                Ok(None) => {}
                Err(msg) => {
                    self.io_tx.send(msg);
                }
            }
        }

        // XXX: this likely hinders performance, so it would be nice to have some sort of lockfree
        //      hashset
        {
            let mut ids = self.ids.lock().unwrap();
            for (dup_id, dup_stat) in possible_dups {
                if !ids.insert(dup_id) {
                    // this was indeed a file we have already seen, adjust the directory's stat
                    // accordingly
                    stat -= dup_stat;
                }
            }
        }

        // send the files in the current directory off to be displayed if "--all" was given and the
        // depth is fine
        if files.len() > 0 && is_valid_depth(self.options, depth) {
            self.io_tx.send(IoMessage::FinishedFiles { files });
        }

        if dirs.len() > 0 {
            let dir_id = FileID::from(&metadata);
            self.io_tx.send(IoMessage::RegisterParent {
                dir_id,
                parent_id,
                path,
                stat,
                subdir_count: dirs.len(),
                depth,
            });

            for (dir_path, dir_metadata) in dirs {
                self.subdir_tx.send(TraversalMessage::Subdir {
                    path: dir_path,
                    metadata: dir_metadata,
                    parent_id: Some(dir_id),
                    depth: depth + 1,
                });
            }
        } else {
            self.io_tx.send(IoMessage::FinishedDir {
                path,
                stat,
                parent_id,
                depth,
            });
        }

        Ok(())
    }

    fn handle_entry(
        &mut self,
        f: io::Result<DirEntry>,
        dirs: &mut Vec<(PathBuf, Metadata)>,
        possible_dups: &mut Vec<(FileID, Stat)>,
        files: &mut Vec<(PathBuf, Stat)>,
    ) -> Result<Option<Stat>, IoMessage> {
        let entry = f.map_err(|e| IoMessage::InvalidEntry { err: e })?;

        let metadata = entry.metadata()
            .map_err(|e| IoMessage::FailedToGetMetadata {
                path: entry.path(),
                err: e,
            })?;

        if metadata.is_dir() {
            dirs.push((entry.path(), metadata));

            Ok(None)
        } else {
            let file_stat = Stat::new(&metadata, self.options);

            // NOTE: pushing here for later rather than checking the HashSet to try to avoid some
            //       lock contention (if we had a lockfree HashSet, we could probably perform the
            //       check here)
            if metadata.is_file() && metadata.nlink() > 1 {
                let file_id = FileID::from(&metadata);
                possible_dups.push((file_id, file_stat));
            }

            if self.options.all {
                files.push((entry.path(), file_stat));
            }

            Ok(Some(file_stat))
        }
    }
}

struct IoWorker<'a, F: Fn(u64) -> String> {
    root_path: PathBuf,
    rx: Option<channel::Receiver<IoMessage>>,
    options: &'a Options,
    convert_size: F,
    time_format_str: &'static str,
    line_separator: &'static str,

    parents: HashMap<FileID, (Option<FileID>, PathBuf, Stat, usize, usize)>,
    last_stat: Option<Stat>,
}

impl<'a, F: Fn(u64) -> String> IoWorker<'a, F> {
    pub fn new(root_path: PathBuf, rx: channel::Receiver<IoMessage>, options: &'a Options, convert_size: F) -> Self {
        // FIXME: when we accept +FORMAT input, this will have the potential to error probably
        //        we could just validate when parsing command-line arguments using a clap validator
        //        potentially to alleviate this issue
        let time_format_str = options.time_style
            .map(|style| match style {
                TimeFormat::FullIso => "%Y-%m-%d %H:%M:%S.%f %z",
                TimeFormat::LongIso => "%Y-%m-%d %H:%M",
                TimeFormat::Iso => "%Y-%m-%d",
            })
            .unwrap_or("%Y-%m-%d %H:%M");

        // FIXME: currently specified in two places
        let line_separator = if options.null { "\0" } else { "\n" };

        Self {
            root_path,
            rx: Some(rx),
            options,
            convert_size,
            time_format_str,
            line_separator,

            parents: HashMap::new(),
            last_stat: None,
        }
    }

    pub fn run(mut self) -> Option<Stat> {
        let stderr_raw = io::stderr();
        let mut stderr = stderr_raw.lock();

        let stdout_raw = io::stdout();
        let mut stdout = stdout_raw.lock();

        // NOTE: these branches need to be as fast as possible to ensure they keep up with the
        //       TraversalWorkers
        for msg in self.rx.take().unwrap() {
            match msg {
                IoMessage::RegisterParent { dir_id, parent_id, path, stat, subdir_count, depth } => {
                    self.parents.insert(dir_id, (parent_id, path, stat, subdir_count, depth));
                }
                IoMessage::FinishedDir { path, stat, parent_id, depth } => {
                    // display directory info
                    if is_valid_depth(self.options, depth) {
                        self.display_entry_info(&mut stdout, &path, &stat);
                    }

                    // let the parent directory know that its subdirectory finished processing
                    if self.update_parent(&mut stdout, path, stat, parent_id, depth) {
                        break;
                    }
                }
                IoMessage::FinishedFiles { files } => {
                    for (file_path, file_stat) in files {
                        self.display_entry_info(&mut stdout, &file_path, &file_stat);
                    }
                }
                IoMessage::CouldNotReadDir { path, stat, parent_id, depth, err } => {
                    safe_writeln!(
                        stderr,
                        "{}: cannot read directory ‘{}‘: {}",
                        //options.program_name,
                        crate::NAME,
                        path.display(),
                        err
                    );

                    // let the parent directory know that its subdirectory finished processing (in
                    // this case by failing to be read)
                    if self.update_parent(&mut stdout, path, stat, parent_id, depth) {
                        break;
                    }
                }
                IoMessage::FailedToGetMetadata { path, err } => {
                    safe_writeln!(
                        stderr,
                        "{}: failed to retrieve metadata for '{}': {}",
                        crate::NAME,
                        path.display(),
                        err
                    );
                }
                IoMessage::InvalidEntry { err } => {
                    safe_writeln!(
                        stderr,
                        "{}: could not read directory entry: {}",
                        crate::NAME,
                        err
                    );
                }
            }
        }

        self.last_stat
    }

    // XXX: atm, true is done with everything, false is continue
    fn update_parent<W: Write>(&mut self, output: &mut W, path: PathBuf, mut stat: Stat, parent_id: Option<FileID>, depth: usize) -> bool {
        if let Some(mut parent_id) = parent_id {
            // check if all the subdirectories in the parent directory have been processed
            // and display stuff for the parent directory if so.  need to loop in case we
            // finish another directory's contents after doing so
            while let HashMapEntry::Occupied(mut entry) = self.parents.entry(parent_id) {
                let (_, _, ref mut parent_stat, ref mut subdir_count, _) = entry.get_mut();

                if !self.options.separate_dirs {
                    *parent_stat += stat;
                }

                self.last_stat = Some(*parent_stat);

                if *subdir_count > 1 {
                    *subdir_count -= 1;
                    break;
                } else {
                    // we have finished processing everything in the parent directory, so display
                    // the parent directory as well
                    let (_, (new_parent, parent_path, parent_stat, _, parent_depth)) = entry.remove_entry();

                    match new_parent {
                        // the parent directory also has a parent directory (the grandparent
                        // directory), so update that one as well
                        Some(new_parent) => {
                            if is_valid_depth(self.options, parent_depth) {
                                self.display_entry_info(output, &parent_path, &parent_stat);
                            }

                            parent_id = new_parent;
                            stat = parent_stat;
                        }
                        None => {
                            // this is the root directory
                            self.display_entry_info(output, &parent_path, &parent_stat);
                            return true;
                        },
                    }
                }
            }

            false
        } else {
            // NOTE: i believe this only occurs if the given directory has no subdirectories
            // this is the root directory
            if self.options.summarize {
                if let Some(ref stat) = self.last_stat {
                    self.display_entry_info(output, &self.root_path, stat);
                }
            }

            true
        }
    }

    // XXX: this function is questionable and may be contributing to the system-dependent errors
    fn entry_size(&self, entry_stat: &Stat) -> u64 {
        if self.options.apparent_size || self.options.bytes {
            entry_stat.size
        } else {
            // C's stat is such that each block is assume to be 512 bytes
            // See: http://linux.die.net/man/2/stat
            entry_stat.blocks * 512
        }
    }

    fn display_entry_info<W: Write>(&self, output: &mut W, path: &Path, entry_stat: &Stat) {
        let size = self.entry_size(entry_stat);

        write!(output, "{}\t", (self.convert_size)(size));
        
        if let Some(_) = self.options.time {
            let secs = (entry_stat.time / 1000) as i64;
            let nsecs = (entry_stat.time % 1000 * 1_000_000) as i32;
            let tm = time::at(Timespec::new(secs, nsecs));

            let time_str = tm.strftime(self.time_format_str).unwrap();
            write!(output, "{}\t", time_str);
        }

        write!(output, "{}{}", path.display(), self.line_separator);
    }
}

fn is_valid_depth(options: &Options, depth: usize) -> bool {
    !options.summarize && (options.max_depth == None || depth <= options.max_depth.unwrap())
}
