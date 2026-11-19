// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) tailable untailable stdlib kqueue Uncategorized unwatch

use crate::args::{FollowMode, Settings};
use crate::follow::files::{FileHandling, PathData};
use crate::paths::{Input, InputKind, MetadataExtTail, PathExtTail};
use crate::{platform, text};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, WatcherKind};
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, channel};
use std::time::{Duration, Instant};
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, set_exit_code};
#[cfg(target_os = "linux")]
use uucore::signals::ensure_stdout_not_broken;
use uucore::{translate, translate_text};

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

    /// Wrapper for `notify::Watcher::watch` to also add the parent directory of `path` if necessary.
    fn watch_with_parent(&mut self, path: &Path, use_polling: bool) -> UResult<()> {
        if self.watch_character_device(path, use_polling)? {
            return Ok(());
        }
        let mut path = path.to_owned();
        #[cfg(target_os = "linux")]
        if path.is_file() {
            // The directory watch reports a write to a symlink's target under
            // the target's name, not the link's, so also watch the link: it is
            // another inode than the directory and nothing is reported twice.
            if path.is_symlink() {
                self.watch(&path, RecursiveMode::NonRecursive)?;
            }
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
                if parent.is_dir() {
                    path = parent.to_owned();
                } else {
                    path = PathBuf::from(".");
                }
            } else {
                return Err(USimpleError::new(
                    1,
                    translate!("tail-error-cannot-watch-parent-directory", "path" => path.quote()),
                ));
            }
        }
        if path.is_relative() {
            path = path.canonicalize()?;
        }

        // for syscalls: 2x "inotify_add_watch" ("filename" and ".") and 1x "inotify_rm_watch"
        self.watch(&path, RecursiveMode::NonRecursive)?;
        Ok(())
    }

    #[cfg(unix)]
    fn watch_character_device(&mut self, path: &Path, use_polling: bool) -> UResult<bool> {
        if !path
            .metadata()
            .is_ok_and(|metadata| metadata.file_type().is_char_device())
        {
            return Ok(false);
        }
        self.watch(path, RecursiveMode::NonRecursive)?;
        // Native events let us notice name replacements promptly. The polling
        // pass also checks device names, so an unreadable parent is harmless.
        // PollWatcher must not watch the parent: it would also hash siblings.
        if !use_polling && let Some(parent) = path.parent() {
            let _ = self.watcher.watch(parent, RecursiveMode::NonRecursive);
        }
        Ok(true)
    }

    #[cfg(not(unix))]
    fn watch_character_device(&mut self, _path: &Path, _use_polling: bool) -> UResult<bool> {
        Ok(false)
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
        }
    }

    pub fn from(settings: &Settings) -> Self {
        Self::new(
            settings.retry,
            settings.follow,
            settings.use_polling,
            FileHandling::from(settings),
            settings.pid.unwrap_or_default(),
        )
    }

    pub fn add_path(
        &mut self,
        path: &Path,
        display_name: &str,
        reader: Option<BufReader<File>>,
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
                PathData::new(reader, metadata, display_name, path.is_symlink())?,
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
        if let Some(watcher_rx) = &mut self.watcher_rx {
            for input in inputs {
                match input.kind() {
                    InputKind::Stdin => (),
                    InputKind::File(path) => {
                        #[cfg(all(unix, not(target_os = "linux")))]
                        if !path.metadata().is_ok_and(|metadata| {
                            metadata.is_file() || metadata.file_type().is_char_device()
                        }) {
                            continue;
                        }
                        let mut path = path.clone();
                        if path.is_relative() {
                            path = std::env::current_dir()?.join(path);
                        }

                        if path.is_tailable() {
                            // Add existing regular files to `Watcher` (InotifyWatcher).
                            watcher_rx.watch_with_parent(&path, self.use_polling)?;
                        } else if let Some(active_parent) = path.parent().filter(|p| p.is_dir()) {
                            // If `path` is not a tailable file, add its parent to `Watcher`.
                            watcher_rx.watch(active_parent, RecursiveMode::NonRecursive)?;
                            // Add symlinks to orphans for retry polling (target may not exist)
                            if path.is_symlink() {
                                self.orphans.push(path);
                            }
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

    /// Open `path` again and refresh device watches and Linux symlink watches.
    /// These watches can still refer to the old target after a name replacement.
    fn reopen(&mut self, path: &Path) -> UResult<()> {
        let was_device = self.files.character_devices().contains(path);
        self.files.update_reader(path)?;
        if was_device && let Some(watcher_rx) = &mut self.watcher_rx {
            let _ = watcher_rx.unwatch(path);
            watcher_rx.watch_with_parent(path, self.use_polling)?;
        }
        #[cfg(target_os = "linux")]
        if !was_device
            && path.is_symlink()
            && let Some(watcher_rx) = &mut self.watcher_rx
        {
            let _ = watcher_rx.unwatch(path);
            watcher_rx.watch(path, RecursiveMode::NonRecursive)?;
        }
        Ok(())
    }

    fn check_device_name(&mut self, path: &Path, settings: &Settings) -> UResult<()> {
        use notify::event::{EventKind, ModifyKind, RemoveKind, RenameMode};

        if !self.follow_name() || !self.files.character_devices().contains(path) {
            return Ok(());
        }
        let kind = match path.metadata() {
            Ok(metadata) => {
                // Compare with the open device: cached path metadata may already
                // describe a replacement that arrived during the previous read.
                let reader = self.files.get(path).reader.as_ref().unwrap();
                if reader.get_ref().metadata()?.file_id_eq(&metadata) {
                    return Ok(());
                }
                EventKind::Modify(ModifyKind::Name(RenameMode::To))
            }
            Err(_) => EventKind::Remove(RemoveKind::Any),
        };
        self.handle_event(
            &notify::Event::new(kind).add_path(path.to_owned()),
            settings,
        )?;
        if self.retry
            && self.files.get(path).reader.is_none()
            && !self.orphans.iter().any(|orphan| orphan == path)
        {
            self.orphans.push(path.to_owned());
        }
        Ok(())
    }

    #[allow(clippy::cognitive_complexity)]
    fn handle_event(
        &mut self,
        event: &notify::Event,
        settings: &Settings,
    ) -> UResult<Vec<PathBuf>> {
        use notify::event::{
            CreateKind, DataChange, EventKind, MetadataKind, ModifyKind, RemoveKind, RenameMode,
        };

        let event_path = event.paths.first().unwrap();
        let mut paths: Vec<PathBuf> = vec![];
        let display_name = self.files.get(event_path).display_name.clone();

        match event.kind {
            // NOTE: `ModifyKind::Any` is emitted by the Windows `ReadDirectoryChangesW`
            // backend (for `FILE_ACTION_MODIFIED`) and by the macOS/BSD `kqueue` backend
            // (for `NOTE_WRITE`) whenever a watched file's contents change. Without it,
            // appends are never followed on those platforms (see GH issue #4827).
            // Linux inotify and the polling fallback report the more specific
            // `Modify(Data(..))`/`Modify(Metadata(..))` instead, so this is a no-op there.
            EventKind::Modify(ModifyKind::Any | ModifyKind::Metadata(MetadataKind::Any | MetadataKind::WriteTime) | ModifyKind::Data(DataChange::Any) | ModifyKind::Name(RenameMode::To)) |
            EventKind::Create(CreateKind::File | CreateKind::Folder | CreateKind::Any) => {
                if let Ok(new_md) = event_path.metadata() {
                    let pd = self.files.get(event_path);
                    // `metadata()` follows symlinks, so under --follow=name a
                    // watched file swapped for a symlink would otherwise be
                    // silently followed to its target, and is treated as
                    // untailable instead. A path that already was a symlink
                    // is followed to its target, as GNU does.
                    let replaced_by_symlink = self.follow_name()
                        && !pd.symlink
                        && event_path
                            .symlink_metadata()
                            .is_ok_and(|m| m.file_type().is_symlink());
                    let is_tailable = !replaced_by_symlink && new_md.is_tailable();
                    if let Some(old_md) = &pd.metadata {
                        if is_tailable {
                            // The path was untailable (missing, or not a
                            // regular file) and has become tailable again:
                            // open the new file and read it from the start.
                            if !old_md.is_tailable() {
                                show_error!(
                                    "{}",
                                    translate!("tail-status-has-become-accessible", "file" => display_name.quote())
                                );
                                self.reopen(event_path)?;
                            } else if pd.reader.is_none() {
                                show_error!(
                                    "{}",
                                    translate!("tail-status-has-appeared-following-new-file", "file" => display_name.quote())
                                );
                                self.reopen(event_path)?;
                            } else if (event.kind == EventKind::Modify(ModifyKind::Name(RenameMode::To))
                                && !pd.reader.as_ref().unwrap().get_ref().metadata()?.file_id_eq(&new_md))
                            || (self.use_polling && !old_md.file_id_eq(&new_md)) {
                                // Compare with the open reader: a device-name check may
                                // have handled the replacement before its native event.
                                show_error!(
                                    "{}",
                                    translate!("tail-status-has-been-replaced-following-new-file", "file" => display_name.quote())
                                );
                                self.reopen(event_path)?;
                            } else if old_md.got_truncated(&new_md)? {
                                // A file that has lost content is read again
                                // from offset zero, since shortening a log
                                // almost always means it was emptied outright.
                                show_error!(
                                    "{}",
                                    translate_text!("tail-status-file-truncated", "file" => display_name)
                                );
                                self.reopen(event_path)?;
                            }
                            paths.push(event_path.clone());
                        } else if !is_tailable && old_md.is_tailable() {
                            if replaced_by_symlink {
                                show_error!(
                                    "{}",
                                    translate!("tail-status-replaced-with-untailable-symlink", "file" => display_name.quote())
                                );
                                self.files.reset_reader(event_path);
                            } else if pd.reader.is_some() {
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
                        self.reopen(event_path)?;
                        paths.push(event_path.clone());
                    } else if settings.retry {
                        if self.follow_descriptor() {
                            show_error!(
                                "{}",
                                translate!("tail-status-replaced-with-untailable-file-giving-up", "file" => display_name.quote())
                            );
                            let _ = self.watcher_rx.as_mut().unwrap().watcher.unwatch(event_path);
                            self.files.remove(event_path);
                            if self.files.no_files_remaining(settings) {
                                return Err(USimpleError::new(1, translate!("tail-no-files-remaining")));
                            }
                        } else {
                            show_error!(
                                "{}",
                                translate!("tail-status-replaced-with-untailable-file", "file" => display_name.quote())
                            );
                        }
                    }
                    self.files.update_metadata(event_path, Some(new_md));
                } else if self.follow_descriptor() && self.files.get(event_path).reader.is_some() {
                    // The link no longer resolves, but the file it led to is
                    // still open and is what --follow=descriptor follows.
                    paths.push(event_path.clone());
                } else if event_path.is_symlink() && settings.retry {
                    // The link no longer resolves. Say so if it was being
                    // followed, and poll it until it resolves again.
                    if self.files.get(event_path).reader.is_some() {
                        report_inaccessible(&display_name);
                    }
                    self.files.reset_reader(event_path);
                    self.orphans.push(event_path.clone());
                }
            }
            EventKind::Remove(RemoveKind::File | RemoveKind::Any)
                | EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                if self.follow_name() {
                    if settings.retry {
                        if self.files.get_mut_metadata(event_path).is_some_and(MetadataExtTail::is_tailable)
                            && self.files.get(event_path).reader.is_some() {
                                report_inaccessible(&display_name);
                        }
                        if !event_path.has_active_parent() && !self.orphans.contains(event_path) {
                            show_error!("{}", translate!("tail-status-directory-containing-watched-file-removed"));
                            show_error!(
                                "{}",
                                translate!("tail-status-backend-cannot-be-used-reverting-to-polling", "backend" => text::BACKEND)
                            );
                            self.orphans.push(event_path.clone());
                            let _ = self.watcher_rx.as_mut().unwrap().unwatch(event_path);
                        } else if event_path.is_symlink() && !self.orphans.contains(event_path) {
                            // The target of the link is gone, and nothing
                            // reports the name it will come back under: poll
                            // the link until it resolves again.
                            self.orphans.push(event_path.clone());
                        }
                    } else {
                        show_error!(
                            "{}: {}",
                            display_name,
                            translate!("tail-no-such-file-or-directory")
                        );
                        if !self.files.files_remaining() {
                            // GNU's tail exits here for both inotify and polling when
                            // --follow=name is used without --retry and no files remain.
                            return Err(USimpleError::new(1, translate!("tail-no-files-remaining")));
                        }
                    }
                    self.files.reset_reader(event_path);
                } else if self.follow_descriptor_retry() {
                    // --retry only effective for the initial open
                    let _ = self.watcher_rx.as_mut().unwrap().unwatch(event_path);
                    self.files.remove(event_path);
                } else if self.use_polling && event.kind == EventKind::Remove(RemoveKind::Any) {
                    // PollWatcher may report a rename as a removal without the new
                    // path, so we cannot reopen the file under its new name.
                }
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::Both))
                /*
                NOTE: For `tail -f a`, keep tracking additions to b after `mv a b`
                (gnu/tests/tail-2/descriptor-vs-rename.sh)
                NOTE: The File/BufReader doesn't need to be updated.
                However, we need to update our `files.map`.
                This can only be done for inotify, because this EventKind does not
                trigger for the PollWatcher.
                BUG: As a result, there's a bug if polling is used:
                $ tail -f file_a ---disable-inotify
                $ mv file_a file_b
                $ echo A >> file_b
                $ echo A >> file_a
                The last append to file_a is printed, however this shouldn't be because
                after the "mv" tail should only follow "file_b".
                TODO: [2022-05; jhscheer] add test for this bug
                */

                if self.follow_descriptor() => {
                    let new_path = event.paths.last().unwrap();
                    paths.push(new_path.clone());

                    let new_data = PathData::from_other_with_path(
                        self.files.remove(event_path), new_path, self.files.nonblocking
                    )?;
                    self.files.insert(
                        new_path,
                        new_data,
                        self.files.get_last().unwrap() == event_path
                    );

                    // Unwatch old path and watch new path
                    let _ = self.watcher_rx.as_mut().unwrap().unwatch(event_path);
                    self.watcher_rx.as_mut().unwrap().watch_with_parent(new_path, self.use_polling)?;
                }
            _ => {}
        }
        Ok(paths)
    }
}

/// A followed name that no longer leads to a file.
fn report_inaccessible(display_name: &str) {
    show_error!(
        "{} {}: {}",
        display_name.quote(),
        translate!("tail-become-inaccessible"),
        translate!("tail-no-such-file-or-directory")
    );
}

#[allow(clippy::cognitive_complexity)]
pub fn follow(mut observer: Observer, settings: &Settings) -> UResult<()> {
    if observer.files.no_files_remaining(settings) && !observer.files.only_stdin_remaining() {
        return Err(USimpleError::new(1, translate!("tail-no-files-remaining")));
    }

    let process = platform::ProcessChecker::new(observer.pid);

    let mut timeout_counter = 0;
    let mut last_device_poll = Instant::now();
    let mut read_devices_again = false;

    // main follow loop
    loop {
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
                // Use metadata() directly instead of exists() + metadata().unwrap()
                // to avoid a TOCTOU race where the file is removed between the two calls.
                if let Ok(md) = new_path.metadata() {
                    let pd = observer.files.get(new_path);
                    if md.is_tailable() && pd.reader.is_none() {
                        show_error!(
                            "{}",
                            translate!("tail-status-has-appeared-following-new-file", "file" => pd.display_name.quote())
                        );
                        observer.files.update_metadata(new_path, Some(md));
                        observer.files.update_reader(new_path)?;
                        read_devices_again |=
                            observer.files.tail_file(new_path, settings.verbose)?;
                        observer
                            .watcher_rx
                            .as_mut()
                            .unwrap()
                            .watch_with_parent(new_path, observer.use_polling)?;
                    }
                }
            }
        }

        // With  -f, sleep for approximately N seconds (default 1.0) between iterations;
        // We wake up if Notify sends an Event or if we wait more than `sleep_sec`.
        let timeout = if read_devices_again {
            Duration::ZERO
        } else if observer.files.character_devices().is_empty() {
            settings.sleep_sec
        } else {
            settings
                .sleep_sec
                .saturating_sub(last_device_poll.elapsed())
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

        // Helper closure to process a single event
        let process_event = |observer: &mut Observer,
                             event: notify::Event,
                             settings: &Settings,
                             paths: &mut Vec<PathBuf>|
         -> UResult<()> {
            // kqueue can report a directory change without the replaced child's
            // name. Recheck devices in that directory before reading their FDs.
            if observer.follow_name() {
                for device in observer.files.character_devices() {
                    if event.paths.iter().any(|path| {
                        device
                            .parent()
                            .is_some_and(|parent| path == parent || path.parent() == Some(parent))
                    }) && !paths.contains(device)
                    {
                        paths.push(device.clone());
                    }
                }
            }
            if event
                .paths
                .first()
                .is_some_and(|path| observer.files.contains_key(path))
            {
                let path = &event.paths[0];
                if observer.files.character_devices().contains(path) {
                    // Device events can describe the old target after a rename.
                    // Check the current name in the print loop; -f keeps its FD.
                    if !paths.contains(path) {
                        paths.push(path.clone());
                    }
                    return Ok(());
                }
                // Handle Event if it is about a path that we are monitoring
                let new_paths = observer.handle_event(&event, settings)?;
                for p in new_paths {
                    if !paths.contains(&p) {
                        paths.push(p);
                    }
                }
            }
            Ok(())
        };

        match rx_result {
            Ok(Ok(event)) => {
                process_event(&mut observer, event, settings, &mut paths)?;

                // Drain any additional pending events to batch them together.
                // This prevents redundant headers when multiple inotify events
                // are queued (e.g., after resuming from SIGSTOP).
                // Multiple iterations with spin_loop hints give the notify
                // background thread chances to deliver pending events.
                for _ in 0..100 {
                    while let Ok(Ok(event)) =
                        observer.watcher_rx.as_mut().unwrap().receiver.try_recv()
                    {
                        process_event(&mut observer, event, settings, &mut paths)?;
                    }
                    // Use both yield and spin hint for broader CPU support
                    std::thread::yield_now();
                    std::hint::spin_loop();
                }
            }
            Ok(Err(notify::Error {
                kind: notify::ErrorKind::Io(ref e),
                paths,
            })) if e.kind() == std::io::ErrorKind::NotFound => {
                if let Some(event_path) = paths.first().filter(|p| observer.files.contains_key(p)) {
                    let _ = observer
                        .watcher_rx
                        .as_mut()
                        .unwrap()
                        .watcher
                        .unwatch(event_path);
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
                // Check if stdout pipe is still open
                #[cfg(target_os = "linux")]
                if let Ok(false) = ensure_stdout_not_broken() {
                    return Ok(());
                }
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

        // Devices don't report new input through filesystem notifications. Poll
        // them on schedule even when other files keep producing events.
        if !observer.files.character_devices().is_empty()
            && (read_devices_again || last_device_poll.elapsed() >= settings.sleep_sec)
        {
            // The polling backend already includes every input above.
            if !observer.use_polling {
                let mut devices: HashSet<_> = observer.files.character_devices().iter().collect();
                for path in &paths {
                    devices.remove(path);
                }
                paths.extend(devices.into_iter().cloned());
            }
            last_device_poll = Instant::now();
        }

        // main print loop
        read_devices_again = false;
        for path in &paths {
            observer.check_device_name(path, settings)?;
            read_devices_again |= observer.files.tail_file(path, settings.verbose)?;
        }

        if timeout_counter == settings.max_unchanged_stats {
            /*
            TODO: [2021-10; jhscheer] implement timeout_counter for each file.
            `--max-unchanged-stats=n` counts per file, not once for the whole run:
            a name that goes n polls (5 by default) without changing should be
            re-opened and its device and inode compared against the handle still
            held for it, which is how the swap behind a rotated log is noticed.
            Only the polling path needs this, and only when following by name.
            */
        }
    }

    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use notify::event::{ModifyKind, RenameMode};
    use std::io::{Seek, SeekFrom};

    #[test]
    fn duplicate_rename_event_preserves_reader_position() {
        let path = std::env::current_exe().unwrap();
        let mut reader = BufReader::new(File::open(&path).unwrap());
        reader.seek(SeekFrom::Start(1)).unwrap();
        let settings = Settings {
            follow: Some(FollowMode::Name),
            ..Settings::default()
        };
        let mut observer = Observer::from(&settings);
        observer
            .add_path(&path, "input", Some(reader), false)
            .unwrap();
        let event = notify::Event::new(notify::EventKind::Modify(ModifyKind::Name(RenameMode::To)))
            .add_path(path.clone());

        observer.handle_event(&event, &settings).unwrap();
        let reader = observer.files.get_mut(&path).reader.as_mut().unwrap();
        assert_eq!(reader.stream_position().unwrap(), 1);
    }
}
