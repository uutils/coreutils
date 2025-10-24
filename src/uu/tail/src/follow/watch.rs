// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) tailable untailable stdlib kqueue Uncategorized unwatch

use crate::args::{FollowMode, Settings};
use crate::follow::files::{BufReadSeek, FileHandling, PathData, WatchSource};
use crate::paths::{Input, InputKind, MetadataExtTail, PathExtTail};
use crate::{platform, text};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, WatcherKind};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, channel};
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, set_exit_code};
use uucore::translate;

use uucore::show_error;

pub struct WatcherRx {
    watcher: Box<dyn Watcher>,
    receiver: Receiver<Result<notify::Event, notify::Error>>,
}

impl WatcherRx {
    fn new(
        watcher: Box<dyn Watcher>,
        receiver: Receiver<Result<notify::Event, notify::Error>>,
    ) -> Self {
        Self { watcher, receiver }
    }

    /// Resolve an event to the actual monitored file path(s) and their watch sources.
    /// This handles mapping parent directory events to the files they affect.
    fn resolve_event_paths(
        &self,
        event: &notify::Event,
        files: &FileHandling,
        _follow_mode: Option<FollowMode>,
    ) -> Vec<(PathBuf, WatchSource)> {
        use notify::event::*;
        
        let event_path = event.paths.first().unwrap();
        let mut resolved = Vec::new();

        // Check if event_path is directly monitored (direct file event)
        if files.contains_key(event_path) {
            resolved.push((event_path.clone(), WatchSource::File));
            return resolved;
        }

        // For parent directory events, find affected monitored files
        // This only applies when parent watching is enabled (Linux + inotify)
        if cfg!(target_os = "linux") {
            // Parent directory event - need to determine which file(s) are affected
            // Strategy: Check which monitored files have actually changed state
            for monitored_path in files.keys() {
                if let Some(parent) = monitored_path.parent() {
                    if parent == event_path {
                        // Check if this file should be included based on event type
                        let should_include = match event.kind {
                            // For Create events, only include files that now exist
                            EventKind::Create(_) => monitored_path.exists(),
                            // For Remove events, only include files that no longer exist
                            EventKind::Remove(_) => !monitored_path.exists(),
                            // For Modify events with Name (rename), check existence change
                            EventKind::Modify(ModifyKind::Name(_)) => true,
                            // For other Modify events, only include if file exists
                            EventKind::Modify(_) => monitored_path.exists(),
                            // For other events, be conservative and include
                            _ => true,
                        };
                        
                        if should_include {
                            resolved.push((monitored_path.clone(), WatchSource::ParentDirectory));
                        }
                    }
                }
            }
        }

        resolved
    }

    /// Wrapper for `notify::Watcher::watch` to also add the parent directory of `path` if necessary.
    /// On Linux with inotify (not polling), we watch BOTH file and parent directory for ALL follow modes.
    /// This is necessary because inotify loses track of a file after it's renamed if we only watch the file.
    /// The notify crate documentation recommends watching the parent directory to handle renames reliably.
    /// Event handling logic must filter and process events appropriately based on follow mode.
    /// NOTE: Tests for --follow=name are disabled on macOS/BSD due to test harness limitations
    /// with capturing output from background processes, but the functionality works correctly.
    fn watch_with_parent(
        &mut self,
        path: &Path,
        _use_polling: bool,
        _follow_name: bool,
    ) -> UResult<()> {
        let mut path = path.to_owned();

        // On Linux with inotify (not polling), watch the parent directory instead of the file.
        // This is a workaround recommended by the notify crate authors to handle renames reliably.
        // NOTE: Watching both file and parent causes duplicate/wrong events, so we only watch parent.
        #[cfg(target_os = "linux")]
        if path.is_file() && !_use_polling {
            /*
            NOTE: Using the parent directory instead of the file is a workaround.
            This workaround follows the recommendation of the notify crate authors:
            > On some platforms, if the `path` is renamed or removed while being watched, behavior may
            > be unexpected. See discussions in [#165] and [#166]. If less surprising behavior is wanted
            > one may non-recursively watch the _parent_ directory as well and manage related events.
            NOTE: Adding both: file and parent results in duplicate/wrong events.
            Tested for notify::InotifyWatcher and for notify::PollWatcher.
            */
            if let Some(parent) = path.parent() {
                // clippy::assigning_clones added with Rust 1.78
                // Rust version = 1.76 on OpenBSD stable/7.5
                #[cfg_attr(not(target_os = "openbsd"), allow(clippy::assigning_clones))]
                if parent.is_dir() {
                    path = parent.to_owned();
                } else {
                    path = PathBuf::from(".");
                }
            } else {
                return Err(USimpleError::new(
                    1,
                    translate!("tail-error-cannot-watch-parent-directory", "path" => path.display()),
                ));
            }
        }

        if path.is_relative() {
            path = path.canonicalize()?;
        }

        // Watch the path (parent directory on Linux, file itself on other platforms)
        self.watch(&path, RecursiveMode::NonRecursive)?;
        Ok(())
    }

    fn watch(&mut self, path: &Path, mode: RecursiveMode) -> UResult<()> {
        self.watcher
            .watch(path, mode)
            .map_err(|err| USimpleError::new(1, err.to_string()))
    }

    fn unwatch(&mut self, path: &Path) -> UResult<()> {
        self.watcher
            .unwatch(path)
            .map_err(|err| USimpleError::new(1, err.to_string()))
    }
}

pub struct Observer {
    /// Whether --retry was given on the command line
    pub retry: bool,

    /// The [`FollowMode`]
    pub follow: Option<FollowMode>,

    /// Indicates whether to use the fallback `polling` method instead of the
    /// platform specific event driven method. Since `use_polling` is subject to
    /// change during runtime it is moved out of [`Settings`].
    pub use_polling: bool,

    pub watcher_rx: Option<WatcherRx>,
    pub orphans: Vec<PathBuf>,
    pub files: FileHandling,

    pub pid: platform::Pid,
    
    /// Simple deduplication: track last message time per file
    last_messages: std::collections::HashMap<PathBuf, std::time::Instant>,
    
    /// Track if the last processed event was synthetic (from fallback logic)
    last_event_was_synthetic: bool,
}

impl Observer {
    pub fn new(
        retry: bool,
        follow: Option<FollowMode>,
        use_polling: bool,
        files: FileHandling,
        pid: platform::Pid,
    ) -> Self {
        let pid = if platform::supports_pid_checks(pid) {
            pid
        } else {
            0
        };

        Self {
            retry,
            follow,
            use_polling,
            watcher_rx: None,
            orphans: Vec::new(),
            files,
            pid,
            last_messages: std::collections::HashMap::new(),
            last_event_was_synthetic: false,
        }
    }

    pub fn from(settings: &Settings) -> Self {
        Self::new(
            settings.retry,
            settings.follow,
            settings.use_polling,
            FileHandling::from(settings),
            settings.pid,
        )
    }
    
    /// Simple deduplication: check if message should be shown (not shown within last 100ms)
    fn should_show_message(&mut self, path: &Path) -> bool {
        let now = std::time::Instant::now();
        if let Some(last_time) = self.last_messages.get(path) {
            if now.duration_since(*last_time).as_millis() < 100 {
                return false; // Too recent, suppress
            }
        }
        self.last_messages.insert(path.to_path_buf(), now);
        true
    }
    

    pub fn add_path(
        &mut self,
        path: &Path,
        display_name: &str,
        reader: Option<Box<dyn BufReadSeek>>,
        update_last: bool,
    ) -> UResult<()> {
        if self.follow.is_some() {
            let path = if path.is_relative() {
                std::env::current_dir()?.join(path)
            } else {
                path.to_owned()
            };
            let metadata = path.metadata().ok();
            self.files.insert(
                &path,
                PathData::new(reader, metadata, display_name),
                update_last,
            );
        }

        Ok(())
    }

    pub fn add_stdin(
        &mut self,
        display_name: &str,
        reader: Option<Box<dyn BufReadSeek>>,
        update_last: bool,
    ) -> UResult<()> {
        if self.follow == Some(FollowMode::Descriptor) {
            return self.add_path(
                &PathBuf::from(text::DEV_STDIN),
                display_name,
                reader,
                update_last,
            );
        }

        Ok(())
    }

    pub fn add_bad_path(
        &mut self,
        path: &Path,
        display_name: &str,
        update_last: bool,
    ) -> UResult<()> {
        if self.retry && self.follow.is_some() {
            return self.add_path(path, display_name, None, update_last);
        }

        Ok(())
    }

    pub fn start(&mut self, settings: &Settings) -> UResult<()> {
        if settings.follow.is_none() {
            return Ok(());
        }

        let (tx, rx) = channel();

        /*
        Watcher is implemented per platform using the best implementation available on that
        platform. In addition to such event driven implementations, a polling implementation
        is also provided that should work on any platform.
        Linux / Android: inotify
        macOS: FSEvents / kqueue
        Windows: ReadDirectoryChangesWatcher
        FreeBSD / NetBSD / OpenBSD / DragonflyBSD: kqueue
        Fallback: polling every n seconds

        NOTE:
        We force the use of kqueue with: features=["macos_kqueue"].
        On macOS only `kqueue` is suitable for our use case because `FSEvents`
        waits for file close util it delivers a modify event. See:
        https://github.com/notify-rs/notify/issues/240
        */

        let watcher: Box<dyn Watcher>;
        let watcher_config = notify::Config::default()
            .with_poll_interval(settings.sleep_sec)
            /*
            NOTE: By enabling compare_contents, performance will be significantly impacted
            as all files will need to be read and hashed at each `poll_interval`.
            However, this is necessary to pass: "gnu/tests/tail-2/F-vs-rename.sh"
            */
            .with_compare_contents(true);
        if self.use_polling || RecommendedWatcher::kind() == WatcherKind::PollWatcher {
            self.use_polling = true; // We have to use polling because there's no supported backend
            watcher = Box::new(notify::PollWatcher::new(tx, watcher_config).unwrap());
        } else {
            let tx_clone = tx.clone();
            match RecommendedWatcher::new(tx, notify::Config::default()) {
                Ok(w) => watcher = Box::new(w),
                Err(e) if e.to_string().starts_with("Too many open files") => {
                    /*
                    NOTE: This ErrorKind is `Uncategorized`, but it is not recommended
                    to match an error against `Uncategorized`
                    NOTE: Could be tested with decreasing `max_user_instances`, e.g.:
                    `sudo sysctl fs.inotify.max_user_instances=64`
                    */
                    show_error!(
                        "{}",
                        translate!("tail-error-backend-cannot-be-used-too-many-files", "backend" => text::BACKEND)
                    );
                    set_exit_code(1);
                    self.use_polling = true;
                    watcher = Box::new(notify::PollWatcher::new(tx_clone, watcher_config).unwrap());
                }
                Err(e) => return Err(USimpleError::new(1, e.to_string())),
            }
        }

        self.watcher_rx = Some(WatcherRx::new(watcher, rx));
        self.init_files(&settings.inputs)?;

        Ok(())
    }

    pub fn follow_descriptor(&self) -> bool {
        self.follow == Some(FollowMode::Descriptor)
    }

    pub fn follow_name(&self) -> bool {
        self.follow == Some(FollowMode::Name)
    }

    pub fn follow_descriptor_retry(&self) -> bool {
        self.follow_descriptor() && self.retry
    }

    pub fn follow_name_retry(&self) -> bool {
        self.follow_name() && self.retry
    }

    fn init_files(&mut self, inputs: &Vec<Input>) -> UResult<()> {
        let use_polling = self.use_polling;
        let follow_name = self.follow_name();
        if let Some(watcher_rx) = &mut self.watcher_rx {
            for input in inputs {
                match input.kind() {
                    InputKind::Stdin => (),
                    InputKind::File(path) => {
                        #[cfg(all(unix, not(target_os = "linux")))]
                        if !path.is_file() {
                            continue;
                        }
                        let mut path = path.clone();
                        if path.is_relative() {
                            path = std::env::current_dir()?.join(path);
                        }

                        if path.is_tailable() {
                            // Add existing regular files to `Watcher` (InotifyWatcher).
                            watcher_rx.watch_with_parent(&path, use_polling, follow_name)?;
                        } else if !path.is_orphan() {
                            // If `path` is not a tailable file, add its parent to `Watcher`.
                            watcher_rx
                                .watch(path.parent().unwrap(), RecursiveMode::NonRecursive)?;
                        } else {
                            // If there is no parent, add `path` to `orphans`.
                            self.orphans.push(path);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::cognitive_complexity)]
    fn handle_event(
        &mut self,
        event: &notify::Event,
        watch_source: WatchSource,
        settings: &Settings,
    ) -> UResult<Vec<PathBuf>> {
        use notify::event::*;

        let event_path = event.paths.first().unwrap();

        // If this is a parent directory event (not a direct file event), return early.
        // The follow() loop will map parent events to monitored files before calling handle_event.
        if !self.files.contains_key(event_path) {
            return Ok(vec![]);
        }

        // For descriptor mode, ignore parent directory events to avoid conflicts
        if self.follow_descriptor() && watch_source == WatchSource::ParentDirectory {
            return Ok(vec![]);
        }

        let mut paths: Vec<PathBuf> = vec![];
        // Safety: we confirmed this path exists in the map above
        let display_name = self.files.get(event_path).display_name.clone();

        match event.kind {
            EventKind::Modify(
                ModifyKind::Metadata(MetadataKind::Any | MetadataKind::WriteTime)
                | ModifyKind::Data(DataChange::Any)
                | ModifyKind::Name(RenameMode::To),
            )
            | EventKind::Create(CreateKind::File | CreateKind::Folder | CreateKind::Any) => {
                if let Ok(new_md) = event_path.metadata() {
                    let is_tailable = new_md.is_tailable();
                    // Safety: we confirmed this path exists in the map above
                    let old_md = self.files.get(event_path).metadata.clone();
                    if let Some(old_md) = &old_md {
                        if is_tailable {
                            // We resume tracking from the start of the file,
                            // assuming it has been truncated to 0. This mimics GNU's `tail`
                            // behavior and is the usual truncation operation for log files.
                            if !old_md.is_tailable() {
                                show_error!(
                                    "{}",
                                    translate!("tail-status-has-become-accessible", "file" => display_name.quote())
                                );
                                self.files.update_reader(event_path)?;
                            } else if self.files.get(event_path).reader.is_none() {
                                // Only show "has appeared" message for real events, not synthetic ones
                                if !self.last_event_was_synthetic {
                                    show_error!(
                                        "{}",
                                        translate!("tail-status-has-appeared-following-new-file", "file" => display_name.quote())
                                    );
                                }
                                self.files.update_reader(event_path)?;
                            } else if event.kind
                                == EventKind::Modify(ModifyKind::Name(RenameMode::To))
                                || !old_md.file_id_eq(&new_md)
                            {
                                // File was replaced (different inode) - only show message on Linux or with polling
                                let should_show = (cfg!(target_os = "linux") || self.use_polling) && self.should_show_message(event_path);
                                if should_show {
                                    show_error!(
                                        "{}",
                                        translate!("tail-status-has-been-replaced-following-new-file", "file" => display_name.quote())
                                    );
                                }
                                
                                self.files.update_reader(event_path)?;
                                
                                // Note: Rename handling is complex and needs careful implementation
                                // For now, we rely on the existing logic to handle file associations
                            } else if old_md.got_truncated(&new_md)? {
                                show_error!(
                                    "{}",
                                    translate!("tail-status-file-truncated", "file" => display_name)
                                );
                                self.files.update_reader_with_positioning(event_path, settings)?;
                                // Re-setup watch after file truncation/recreation
                                if self.follow_name() {
                                    let use_polling = self.use_polling;
                                    let follow_name = self.follow_name();
                                    self.watcher_rx.as_mut().unwrap().watch_with_parent(
                                        event_path,
                                        use_polling,
                                        follow_name,
                                    )?;
                                }
                            }
                            paths.push(event_path.clone());
                        } else if !is_tailable && old_md.is_tailable() {
                            if self.files.get(event_path).reader.is_some() {
                                self.files.reset_reader(event_path);
                            } else {
                                show_error!(
                                    "{}",
                                    translate!("tail-status-replaced-with-untailable-file", "file" => display_name.quote())
                                );
                            }
                        }
                    } else if is_tailable {
                        show_error!(
                            "{}",
                            translate!("tail-status-has-appeared-following-new-file", "file" => display_name.quote())
                        );
                        self.files.update_reader(event_path)?;
                        paths.push(event_path.clone());
                    } else if settings.retry {
                        if self.follow_descriptor() {
                            show_error!(
                                "{}",
                                translate!("tail-status-replaced-with-untailable-file-giving-up", "file" => display_name.quote())
                            );
                            let _ = self
                                .watcher_rx
                                .as_mut()
                                .unwrap()
                                .watcher
                                .unwatch(event_path);
                            // Safety: we confirmed this path exists in the map above
                            self.files.remove(event_path);
                            if self.files.no_files_remaining(settings) {
                                return Err(USimpleError::new(
                                    1,
                                    translate!("tail-no-files-remaining"),
                                ));
                            }
                        } else {
                            show_error!(
                                "{}",
                                translate!("tail-status-replaced-with-untailable-file", "file" => display_name.quote())
                            );
                        }
                    }
                    self.files.update_metadata(event_path, Some(new_md));
                }
            }
            EventKind::Remove(RemoveKind::File | RemoveKind::Any)
            | EventKind::Modify(ModifyKind::Name(RenameMode::Any))
            | EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                // In descriptor mode with inotify, handle rename events specially
                if self.follow_descriptor()
                    && !self.use_polling
                    && watch_source == WatchSource::File
                {
                    // File was renamed or watch was invalidated
                    // Switch to polling fallback since inotify watch is now invalid
                    self.files.get_mut(event_path).fallback_to_polling = true;
                    let _ = self
                        .watcher_rx
                        .as_mut()
                        .unwrap()
                        .watcher
                        .unwatch(event_path);
                    // Don't remove from files map - FD is still valid
                } else if self.follow_name() {
                    if settings.retry {
                        // Safety: we confirmed this path exists in the map above
                        if let Some(old_md) = self.files.get_mut_metadata(event_path) {
                            // Safety: we confirmed this path exists in the map above
                            if old_md.is_tailable() && self.files.get(event_path).reader.is_some() {
                                show_error!(
                                    "{}",
                                    translate!("tail-status-file-became-inaccessible", "file" => display_name.quote(), "become_inaccessible" => translate!("tail-become-inaccessible"), "no_such_file" => translate!("tail-no-such-file-or-directory"))
                                );
                            }
                        }
                        if event_path.is_orphan() && !self.orphans.contains(event_path) {
                            show_error!(
                                "{}",
                                translate!("tail-status-directory-containing-watched-file-removed")
                            );
                            show_error!(
                                "{}",
                                translate!("tail-status-backend-cannot-be-used-reverting-to-polling", "backend" => text::BACKEND)
                            );
                            self.orphans.push(event_path.clone());
                            let _ = self.watcher_rx.as_mut().unwrap().unwatch(event_path);
                        }
                    } else {
                        show_error!(
                            "{}",
                            translate!("tail-status-file-no-such-file", "file" => display_name, "no_such_file" => translate!("tail-no-such-file-or-directory"))
                        );
                        if !self.files.files_remaining() && self.use_polling {
                            // NOTE: GNU's tail exits here for `---disable-inotify`
                            return Err(USimpleError::new(
                                1,
                                translate!("tail-no-files-remaining"),
                            ));
                        }
                    }
                    self.files.reset_reader(event_path);
                } else if self.follow_descriptor_retry() {
                    // --retry only effective for the initial open
                    let _ = self.watcher_rx.as_mut().unwrap().unwatch(event_path);
                    // Safety: we confirmed this path exists in the map above
                    self.files.remove(event_path);
                } else if self.use_polling && event.kind == EventKind::Remove(RemoveKind::Any) {
                    /*
                    BUG: The watched file was removed. Since we're using Polling, this
                    could be a rename. We can't tell because `notify::PollWatcher` doesn't
                    recognize renames properly.
                    Ideally we want to call seek to offset 0 on the file handle.
                    But because we only have access to `PathData::reader` as `BufRead`,
                    we cannot seek to 0 with `BufReader::seek_relative`.
                    Also because we don't have the new name, we cannot work around this
                    by simply reopening the file.
                    */
                }
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                /*
                NOTE: For `tail -f a`, keep tracking additions to b after `mv a b`
                (gnu/tests/tail-2/descriptor-vs-rename.sh)
                NOTE: The File/BufReader doesn't need to be updated because we're following
                the file descriptor, which remains valid after a rename.

                For --follow=descriptor mode with direct file watching (not parent directory),
                inotify only provides the old path in the event, not the new path.
                Since we're following the descriptor anyway, we don't need to update the HashMap key.
                We just continue using the original path as the key and the file descriptor stays valid.

                For --follow=name mode or parent directory watching, this would need different handling.

                BUG: As a result, there's a bug if polling is used:
                $ tail -f file_a ---disable-inotify
                $ mv file_a file_b
                $ echo A >> file_b
                $ echo A >> file_a
                The last append to file_a is printed, however this shouldn't be because
                after the "mv" tail should only follow "file_b".
                TODO: [2022-05; jhscheer] add test for this bug
                */

                if self.follow_descriptor() && watch_source == WatchSource::File {
                    // For descriptor mode with direct file watching, we don't need to update
                    // the HashMap because:
                    // 1. The file descriptor remains valid after rename
                    // 2. The inotify event only contains the old path, not the new path
                    // 3. We're following the descriptor, not the name
                    //
                    // However, after rename the inotify watch becomes invalid (path-based).
                    // Switch to periodic FD polling to catch new writes.
                    if !self.use_polling {
                        // Mark for polling fallback
                        self.files.get_mut(event_path).fallback_to_polling = true;
                        // Optional: unwatch the path since it's no longer valid
                        let _ = self
                            .watcher_rx
                            .as_mut()
                            .unwrap()
                            .watcher
                            .unwatch(event_path);
                    }
                    // Just add the path to the list for reading new content.
                    paths.push(event_path.clone());
                }
            }
            _ => {
                // Catch-all for any other events - handle descriptor mode fallback
                if self.follow_descriptor()
                    && !self.use_polling
                    && watch_source == WatchSource::File
                {
                    // For ANY unexpected events in descriptor mode with inotify that might indicate
                    // the file was modified or renamed, switch to polling fallback as a safety measure.
                    // This ensures we never miss data due to unexpected event types.
                    // We check if the file path still exists - if it doesn't, it was likely renamed/moved.
                    if !event_path.exists() {
                        self.files.get_mut(event_path).fallback_to_polling = true;
                        let _ = self
                            .watcher_rx
                            .as_mut()
                            .unwrap()
                            .watcher
                            .unwatch(event_path);
                        // Add path to reading list to try reading any remaining data from FD
                        paths.push(event_path.clone());
                    }
                }
            }
        }
        Ok(paths)
    }
}

#[allow(clippy::cognitive_complexity)]
pub fn follow(mut observer: Observer, settings: &Settings) -> UResult<()> {
    // Debug: Log that follow function was called

    if observer.files.no_files_remaining(settings) && !observer.files.only_stdin_remaining() {
        return Err(USimpleError::new(1, translate!("tail-no-files-remaining")));
    }

    let mut process = platform::ProcessChecker::new(observer.pid);

    let mut timeout_counter = 0;

    // main follow loop
    loop {
        let mut _read_some = false;

        // If `--pid=p`, tail checks whether process p
        // is alive at least every `--sleep-interval=N` seconds
        if settings.follow.is_some() && observer.pid != 0 && process.is_dead() {
            // p is dead, tail will also terminate
            break;
        }

        // For `-F` we need to poll if an orphan path becomes available during runtime.
        // If a path becomes an orphan during runtime, it will be added to orphans.
        // To be able to differentiate between the cases of test_retry8 and test_retry9,
        // here paths will not be removed from orphans if the path becomes available.
        if observer.follow_name_retry() {
            for new_path in &observer.orphans {
                if new_path.exists() && observer.files.contains_key(new_path) {
                    // Safety: we just confirmed this path exists in the map above
                    let pd = observer.files.get(new_path);
                    let md = new_path.metadata().unwrap();
                    if md.is_tailable() && pd.reader.is_none() {
                        show_error!(
                            "{}",
                            translate!("tail-status-has-appeared-following-new-file", "file" => pd.display_name.quote())
                        );
                        observer.files.update_metadata(new_path, Some(md));
                        observer.files.update_reader(new_path)?;
                        _read_some = observer.files.tail_file(new_path, settings.verbose)?;
                        let use_polling = observer.use_polling;
                        let follow_name = observer.follow_name();
                        observer.watcher_rx.as_mut().unwrap().watch_with_parent(
                            new_path,
                            use_polling,
                            follow_name,
                        )?;
                    }
                }
            }
        }

        // With  -f, sleep for approximately N seconds (default 1.0) between iterations;
        // We wake up if Notify sends an Event or if we wait more than `sleep_sec`.
        // If any files are in polling fallback mode (after rename in descriptor mode),
        // use a shorter timeout (100ms) to ensure responsive polling.
        let poll_interval = std::time::Duration::from_millis(100);
        let timeout = if observer.files.has_polling_fallback() {
            poll_interval.min(settings.sleep_sec)
        } else {
            settings.sleep_sec
        };

        let rx_result = observer
            .watcher_rx
            .as_mut()
            .unwrap()
            .receiver
            .recv_timeout(timeout);

        if rx_result.is_ok() {
            timeout_counter = 0;
        }

        let mut paths = vec![]; // Paths worth checking for new content to print
        match rx_result {
            Ok(Ok(event)) => {
                // Use new event resolution logic to properly handle parent directory events
                let resolved_paths = observer.watcher_rx.as_ref().unwrap().resolve_event_paths(
                    &event,
                    &observer.files,
                    observer.follow,
                );

                for (file_path, watch_source) in resolved_paths {
                    // Create a modified event with the correct file path for handle_event
                    let mut modified_event = event.clone();
                    modified_event.paths = vec![file_path.clone()];

                    // Handle the event with watch source information
                    let event_paths =
                        observer.handle_event(&modified_event, watch_source, settings)?;
                    paths.extend(event_paths);
                    
                    // Reset synthetic flag after processing real event
                    observer.last_event_was_synthetic = false;
                }

                // Fallback: if no paths were resolved but we're in follow=name mode,
                // check for file recreation
                if paths.is_empty() && observer.follow_name() {
                    for monitored_path in observer.files.keys() {
                        if monitored_path.exists() {
                            let mut modified_event = event.clone();
                            modified_event.paths = vec![monitored_path.clone()];
                            modified_event.kind =
                                notify::EventKind::Create(notify::event::CreateKind::File);
                            
                            // Mark this as a synthetic event
                            observer.last_event_was_synthetic = true;
                            let event_paths = observer.handle_event(
                                &modified_event,
                                WatchSource::File,
                                settings,
                            )?;
                            paths.extend(event_paths);
                            break;
                        }
                    }
                }
            }
            Ok(Err(notify::Error {
                kind: notify::ErrorKind::Io(ref e),
                paths,
            })) if e.kind() == std::io::ErrorKind::NotFound => {
                if let Some(event_path) = paths.first() {
                    if observer.files.contains_key(event_path) {
                        let _ = observer
                            .watcher_rx
                            .as_mut()
                            .unwrap()
                            .watcher
                            .unwatch(event_path);
                    }
                }
            }
            Ok(Err(notify::Error {
                kind: notify::ErrorKind::MaxFilesWatch,
                ..
            })) => {
                return Err(USimpleError::new(
                    1,
                    translate!("tail-error-backend-resources-exhausted", "backend" => text::BACKEND),
                ));
            }
            Ok(Err(e)) => {
                return Err(USimpleError::new(
                    1,
                    translate!("tail-error-notify-error", "error" => e),
                ));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                timeout_counter += 1;
                // Poll all FDs marked for fallback (after rename in descriptor mode)
                let _ = observer.files.poll_all_fds(settings.verbose)?;
            }
            Err(e) => {
                return Err(USimpleError::new(
                    1,
                    translate!("tail-error-recv-timeout-error", "error" => e),
                ));
            }
        }

        if observer.use_polling && settings.follow.is_some() {
            // Consider all files to potentially have new content.
            // This is a workaround because `Notify::PollWatcher`
            // does not recognize the "renaming" of files.
            paths = observer.files.keys().cloned().collect::<Vec<_>>();
        }

        // main print loop
        for path in &paths {
            _read_some = observer.files.tail_file(path, settings.verbose)?;
        }

        if timeout_counter == settings.max_unchanged_stats {
            /*
            TODO: [2021-10; jhscheer] implement timeout_counter for each file.
            '--max-unchanged-stats=n'
            When tailing a file by name, if there have been n (default n=5) consecutive iterations
            for which the file has not changed, then open/fstat the file to determine if that file
            name is still associated with the same device/inode-number pair as before. When
            following a log file that is rotated, this is approximately the number of seconds
            between when tail prints the last pre-rotation lines and when it prints the lines that
            have accumulated in the new log file. This option is meaningful only when polling
            (i.e., without inotify) and when following by name.
            */
        }
    }

    Ok(())
}
