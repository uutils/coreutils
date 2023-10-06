// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (path) eacces

use clap::{builder::ValueParser, crate_version, parser::ValueSource, Arg, ArgAction, Command};
use std::ffi::OsString;
use std::fs::{self, File, Metadata, ReadDir};
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::{format_usage, help_about, help_section, help_usage, prompt_yes, show, show_if_err};

#[derive(Eq, PartialEq, Clone, Copy)]
/// Enum, determining when the `rm` will prompt the user about the file deletion
pub enum InteractiveMode {
    /// Never prompt
    Never,
    /// Prompt once before removing more than three files, or when removing
    /// recursively.
    Once,
    /// Prompt before every removal
    Always,
    /// Prompt only on write-protected files
    PromptProtected,
}

/// Options for the `rm` command
///
/// All options are public so that the options can be programmatically
/// constructed by other crates, such as Nushell. That means that this struct
/// is part of our public API. It should therefore not be changed without good
/// reason.
///
/// The fields are documented with the arguments that determine their value.
pub struct Options {
    /// `-f`, `--force`
    pub force: bool,
    /// Iterative mode, determines when the command will prompt.
    ///
    /// Set by the following arguments:
    /// - `-i`: [`InteractiveMode::Always`]
    /// - `-I`: [`InteractiveMode::Once`]
    /// - `--interactive`: sets one of the above or [`InteractiveMode::Never`]
    /// - `-f`: implicitly sets [`InteractiveMode::Never`]
    ///
    /// If no other option sets this mode, [`InteractiveMode::PromptProtected`]
    /// is used
    pub interactive: InteractiveMode,
    #[allow(dead_code)]
    /// `--one-file-system`
    pub one_fs: bool,
    /// `--preserve-root`/`--no-preserve-root`
    pub preserve_root: bool,
    /// `-r`, `--recursive`
    pub recursive: bool,
    /// `-d`, `--dir`
    pub dir: bool,
    /// `-v`, `--verbose`
    pub verbose: bool,
}

const ABOUT: &str = help_about!("rm.md");
const USAGE: &str = help_usage!("rm.md");
const AFTER_HELP: &str = help_section!("after help", "rm.md");

static OPT_DIR: &str = "dir";
static OPT_INTERACTIVE: &str = "interactive";
static OPT_FORCE: &str = "force";
static OPT_NO_PRESERVE_ROOT: &str = "no-preserve-root";
static OPT_ONE_FILE_SYSTEM: &str = "one-file-system";
static OPT_PRESERVE_ROOT: &str = "preserve-root";
static OPT_PROMPT: &str = "prompt";
static OPT_PROMPT_MORE: &str = "prompt-more";
static OPT_RECURSIVE: &str = "recursive";
static OPT_VERBOSE: &str = "verbose";
static PRESUME_INPUT_TTY: &str = "-presume-input-tty";

static ARG_FILES: &str = "files";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().after_help(AFTER_HELP).try_get_matches_from(args)?;

    let files: Vec<&Path> = matches
        .get_many::<OsString>(ARG_FILES)
        .map(|v| v.map(Path::new).collect())
        .unwrap_or_default();

    let force_flag = matches.get_flag(OPT_FORCE);

    // If -f(--force) is before any -i (or variants) we want prompts else no prompts
    let force_prompt_never: bool = force_flag && {
        let force_index = matches.index_of(OPT_FORCE).unwrap_or(0);
        ![OPT_PROMPT, OPT_PROMPT_MORE, OPT_INTERACTIVE]
            .iter()
            .any(|flag| {
                matches.value_source(flag) == Some(ValueSource::CommandLine)
                    && matches.index_of(flag).unwrap_or(0) > force_index
            })
    };

    if files.is_empty() && !force_flag {
        // Still check by hand and not use clap
        // Because "rm -f" is a thing
        return Err(UUsageError::new(1, "missing operand"));
    } else {
        let options = Options {
            force: force_flag,
            interactive: {
                if force_prompt_never {
                    InteractiveMode::Never
                } else if matches.get_flag(OPT_PROMPT) {
                    InteractiveMode::Always
                } else if matches.get_flag(OPT_PROMPT_MORE) {
                    InteractiveMode::Once
                } else if matches.contains_id(OPT_INTERACTIVE) {
                    match matches.get_one::<String>(OPT_INTERACTIVE).unwrap().as_str() {
                        "never" => InteractiveMode::Never,
                        "once" => InteractiveMode::Once,
                        "always" => InteractiveMode::Always,
                        val => {
                            return Err(USimpleError::new(
                                1,
                                format!("Invalid argument to interactive ({val})"),
                            ))
                        }
                    }
                } else {
                    InteractiveMode::PromptProtected
                }
            },
            one_fs: matches.get_flag(OPT_ONE_FILE_SYSTEM),
            preserve_root: !matches.get_flag(OPT_NO_PRESERVE_ROOT),
            recursive: matches.get_flag(OPT_RECURSIVE),
            dir: matches.get_flag(OPT_DIR),
            verbose: matches.get_flag(OPT_VERBOSE),
        };
        if options.interactive == InteractiveMode::Once && (options.recursive || files.len() > 3) {
            let msg: String = format!(
                "remove {} {}{}",
                files.len(),
                if files.len() > 1 {
                    "arguments"
                } else {
                    "argument"
                },
                if options.recursive {
                    " recursively?"
                } else {
                    "?"
                }
            );
            if !prompt_yes!("{}", msg) {
                return Ok(());
            }
        }

        remove(files, options);
    }
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(OPT_FORCE)
                .short('f')
                .long(OPT_FORCE)
                .help("ignore nonexistent files and arguments, never prompt")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_PROMPT)
                .short('i')
                .help("prompt before every removal")
                .overrides_with_all([OPT_PROMPT_MORE, OPT_INTERACTIVE])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_PROMPT_MORE)
                .short('I')
                .help("prompt once before removing more than three files, or when removing recursively. \
                Less intrusive than -i, while still giving some protection against most mistakes")
                .overrides_with_all([OPT_PROMPT, OPT_INTERACTIVE])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_INTERACTIVE)
                .long(OPT_INTERACTIVE)
                .help(
                    "prompt according to WHEN: never, once (-I), or always (-i). Without WHEN, \
                    prompts always",
                )
                .value_name("WHEN")
                .num_args(0..=1)
                .require_equals(true)
                .default_missing_value("always")
                .overrides_with_all([OPT_PROMPT, OPT_PROMPT_MORE]),
        )
        .arg(
            Arg::new(OPT_ONE_FILE_SYSTEM)
                .long(OPT_ONE_FILE_SYSTEM)
                .help(
                    "when removing a hierarchy recursively, skip any directory that is on a file \
                    system different from that of the corresponding command line argument (NOT \
                    IMPLEMENTED)",
                ).action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_NO_PRESERVE_ROOT)
                .long(OPT_NO_PRESERVE_ROOT)
                .help("do not treat '/' specially")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_PRESERVE_ROOT)
                .long(OPT_PRESERVE_ROOT)
                .help("do not remove '/' (default)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_RECURSIVE)
                .short('r')
                .visible_short_alias('R')
                .long(OPT_RECURSIVE)
                .help("remove directories and their contents recursively")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_DIR)
                .short('d')
                .long(OPT_DIR)
                .help("remove empty directories")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_VERBOSE)
                .short('v')
                .long(OPT_VERBOSE)
                .help("explain what is being done")
                .action(ArgAction::SetTrue),
        )
        // From the GNU source code:
        // This is solely for testing.
        // Do not document.
        // It is relatively difficult to ensure that there is a tty on stdin.
        // Since rm acts differently depending on that, without this option,
        // it'd be harder to test the parts of rm that depend on that setting.
        // In contrast with Arg::long, Arg::alias does not strip leading
        // hyphens. Therefore it supports 3 leading hyphens.
        .arg(
            Arg::new(PRESUME_INPUT_TTY)
                .long("presume-input-tty")
                .alias(PRESUME_INPUT_TTY)
                .hide(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .num_args(1..)
                .value_hint(clap::ValueHint::AnyPath),
        )
}

// TODO: implement one-file-system (this may get partially implemented in walkdir)
/// Remove (or unlink) the given files
///
/// Returns true if it has encountered an error.
///
/// Behavior is determined by the `options` parameter, see [`Options`] for
/// details.
pub fn remove(files: Vec<&Path>, options: Options) {
    for result in remove_iter(files.into_iter(), options) {
        show_if_err!(result);
    }
}

/// Remove (or unlink) the given files
///
/// The return type is an iterator of errors. Calling `next` triggers the
/// removal of the next file. This allows callers to choose whether they
/// want to
///  - print errors,
///  - stop execution on the first error,
///  - or something else.
///
/// Behavior is determined by the `options` parameter, see [`Options`] for
/// details.
pub fn remove_iter<'a, I>(files: I, options: Options) -> Remover<'a, I>
where
    I: Iterator<Item = &'a Path>,
{
    Remover::new(files, options)
}

pub struct Remover<'a, I: Iterator<Item = &'a Path>> {
    recursive_remover: Option<RecursiveRemover>,
    items: I,
    options: Options,
}

impl<'a, I: Iterator<Item = &'a Path>> Remover<'a, I> {
    fn new(items: I, options: Options) -> Self {
        Remover {
            recursive_remover: None,
            items,
            options,
        }
    }

    fn single_item(&mut self, path: &Path) -> UResult<()> {
        let is_dir = path.symlink_metadata().map_or(false, |md| md.is_dir());

        if is_dir {
            let is_root = path.has_root() && path.parent().is_none();
            if is_root && self.options.preserve_root {
                return Err(USimpleError::new(
                    1,
                    "it is dangerous to operate recursively on '/'\n\
                     use --no-preserve-root to override this failsafe",
                ));
            }

            // Maybe we'll start recursing, if the user decided not to descend
            if self.options.recursive && !dir_is_empty(path)? {
                let mut rec = RecursiveRemover::new();
                rec.maybe_recurse(path, &self.options)
                    .map_err_context(|| format!("cannot descend into {}", path.quote()))?;
                // The remover was initialized with one element, so it must return one
                let ret = rec.next(&self.options);
                self.recursive_remover = Some(rec);
                if let Some(ret) = ret {
                    return ret;
                } else {
                    return Ok(());
                }
            } else if self.options.dir || self.options.recursive {
                return remove_dir(path, &self.options);
            }
        }

        if is_symlink_dir(path) {
            remove_dir(path, &self.options)
        } else {
            remove_file(path, &self.options)
        }
    }
}

fn dir_is_empty(path: &Path) -> io::Result<bool> {
    Ok(fs::read_dir(path)?.next().is_none())
}

impl<'a, I: Iterator<Item = &'a Path>> Iterator for Remover<'a, I> {
    type Item = UResult<()>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(r) = &mut self.recursive_remover {
            if let Some(x) = r.next(&self.options) {
                return Some(x);
            } else {
                self.recursive_remover = None;
            }
        }

        if let Some(p) = self.items.next() {
            return Some(self.single_item(p));
        };

        None
    }
}

struct StackItem {
    path: PathBuf,
    read_dir: ReadDir,
    remove_self: bool,
}

struct RecursiveRemover {
    stack: Vec<StackItem>,
}

impl RecursiveRemover {
    fn new() -> Self {
        Self { stack: Vec::new() }
    }

    fn maybe_recurse(&mut self, path: &Path, options: &Options) -> io::Result<()> {
        if prompt_descend(path, options) {
            self.stack.push(StackItem {
                read_dir: fs::read_dir(path)?,
                path: path.into(),
                remove_self: true,
            });
        } else {
            self.do_not_remove_parents();
        }
        Ok(())
    }

    fn do_not_remove_parents(&mut self) {
        for s in &mut self.stack {
            s.remove_self = false;
        }
    }

    /// Only call when the stack is non-empty!
    fn remove_one_from_stack(&mut self, options: &Options) -> UResult<()> {
        debug_assert!(!self.stack.is_empty());

        loop {
            match self.stack.last_mut().unwrap().read_dir.next() {
                Some(item) => {
                    // Bubble up any IO errors.
                    let item = item?;
                    let path = item.path();

                    // We got an item so now we need to handle that:
                    //  - If it's a directory, push it to the stack and recurse
                    //  - If it's a file, remove it
                    let ft = item.file_type()?;
                    if ft.is_dir() {
                        if fs::read_dir(&path)?.next().is_none() {
                            return remove_dir(&path, options);
                        }
                        self.maybe_recurse(&path, options)
                            .map_err_context(|| format!("cannot recurse {}", path.quote()))?;
                        // We continue the loop, which will now use our new stack item
                        // This is an explicit tail call kind of thing.
                        continue;
                    } else if is_symlink_dir(&path) {
                        return remove_dir(&path, options);
                    } else {
                        return remove_file(&path, options);
                    }
                }
                None => {
                    // Unwrap is fine because `last_mut` succeeded.
                    let stack_item = self.stack.pop().unwrap();
                    if stack_item.remove_self {
                        return remove_dir(&stack_item.path, options);
                    } else {
                        return Ok(());
                    }
                }
            }
        }
    }

    fn next(&mut self, options: &Options) -> Option<UResult<()>> {
        // We keep a stack of read dirs to keep track of where we are
        // The last item of the stack is the current directory
        // So get the next item from the last item on the stack
        if self.stack.is_empty() {
            None
        } else {
            let res = self.remove_one_from_stack(options);
            if res.is_err() {
                self.do_not_remove_parents();
            }
            Some(res)
        }
    }
}

fn is_missing_file_error(result: &io::Result<()>) -> bool {
    let Err(e) = result else {
        return false;
    };
    let Some(err) = e.raw_os_error() else {
        return false;
    };
    matches!(
        err,
        libc::EILSEQ | libc::EINVAL | libc::ENOENT | libc::ENOTDIR
    )
}

fn remove_dir(path: &Path, options: &Options) -> UResult<()> {
    if !prompt_dir(path, options) {
        return Ok(());
    }

    let res = fs::remove_dir(path);

    if options.force && is_missing_file_error(&res) {
        return Ok(());
    }

    res.map_err_context(|| format!("cannot remove {}", path.quote()))?;

    if options.verbose {
        println!("removed directory {}", normalize(path).quote());
    }

    Ok(())
}

fn remove_file(path: &Path, options: &Options) -> UResult<()> {
    if !prompt_file(path, options) {
        return Ok(());
    }

    let res = fs::remove_file(path);

    if options.force && is_missing_file_error(&res) {
        return Ok(());
    }

    res.map_err_context(|| format!("cannot remove {}", path.quote()))?;

    if options.verbose {
        println!("removed {}", normalize(path).quote());
    }

    Ok(())
}

fn prompt_dir(path: &Path, options: &Options) -> bool {
    // If interactive is Never we never want to send prompts
    if options.interactive == InteractiveMode::Never {
        return true;
    }

    // We can't use metadata.permissions.readonly for directories because it only works on files
    // So we have to handle whether a directory is writable manually
    if let Ok(metadata) = fs::metadata(path) {
        handle_writable_directory(path, options, &metadata)
    } else {
        true
    }
}

#[allow(clippy::cognitive_complexity)]
fn prompt_file(path: &Path, options: &Options) -> bool {
    // If interactive is Never we never want to send prompts
    if options.interactive == InteractiveMode::Never {
        return true;
    }
    // If interactive is Always we want to check if the file is symlink to prompt the right message
    if options.interactive == InteractiveMode::Always {
        if let Ok(metadata) = fs::symlink_metadata(path) {
            if metadata.is_symlink() {
                return prompt_yes!("remove symbolic link {}?", path.quote());
            }
        }
    }
    // File::open(path) doesn't open the file in write mode so we need to use file options to open it in also write mode to check if it can written too
    match File::options().read(true).write(true).open(path) {
        Ok(file) => {
            if let Ok(metadata) = file.metadata() {
                if metadata.permissions().readonly() {
                    if metadata.len() == 0 {
                        prompt_yes!(
                            "remove write-protected regular empty file {}?",
                            path.quote()
                        )
                    } else {
                        prompt_yes!("remove write-protected regular file {}?", path.quote())
                    }
                } else if options.interactive == InteractiveMode::Always {
                    if metadata.len() == 0 {
                        prompt_yes!("remove regular empty file {}?", path.quote())
                    } else {
                        prompt_yes!("remove file {}?", path.quote())
                    }
                } else {
                    true
                }
            } else {
                true
            }
        }
        Err(err) => {
            if err.kind() == ErrorKind::PermissionDenied {
                match fs::metadata(path) {
                    Ok(metadata) if metadata.len() == 0 => {
                        prompt_yes!(
                            "remove write-protected regular empty file {}?",
                            path.quote()
                        )
                    }
                    _ => prompt_yes!("remove write-protected regular file {}?", path.quote()),
                }
            } else {
                true
            }
        }
    }
}

// For directories finding if they are writable or not is a hassle. In Unix we can use the built-in rust crate to to check mode bits. But other os don't have something similar afaik
// Most cases are covered by keep eye out for edge cases
#[cfg(unix)]
fn handle_writable_directory(path: &Path, options: &Options, metadata: &Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let mode = metadata.permissions().mode();
    // Check if directory has user write permissions
    // Why is S_IWUSR showing up as a u16 on macos?
    #[allow(clippy::unnecessary_cast)]
    let user_writable = (mode & (libc::S_IWUSR as u32)) != 0;
    if !user_writable {
        prompt_yes!("remove write-protected directory {}?", path.quote())
    } else if options.interactive == InteractiveMode::Always {
        prompt_yes!("remove directory {}?", path.quote())
    } else {
        true
    }
}

// For windows we can use windows metadata trait and file attributes to see if a directory is readonly
#[cfg(windows)]
fn handle_writable_directory(path: &Path, options: &Options, metadata: &Metadata) -> bool {
    use std::os::windows::prelude::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_READONLY;
    let not_user_writable = (metadata.file_attributes() & FILE_ATTRIBUTE_READONLY) != 0;
    if not_user_writable {
        prompt_yes!("remove write-protected directory {}?", path.quote())
    } else if options.interactive == InteractiveMode::Always {
        prompt_yes!("remove directory {}?", path.quote())
    } else {
        true
    }
}

// I have this here for completeness but it will always return "remove directory {}" because metadata.permissions().readonly() only works for file not directories
#[cfg(not(windows))]
#[cfg(not(unix))]
fn handle_writable_directory(path: &Path, options: &Options, metadata: &Metadata) -> bool {
    if options.interactive == InteractiveMode::Always {
        prompt_yes!("remove directory {}?", path.quote())
    } else {
        true
    }
}

fn prompt_descend(path: &Path, options: &Options) -> bool {
    if options.interactive == InteractiveMode::Always {
        prompt_yes!("descend into directory {}?", path.quote())
    } else {
        true
    }
}

fn normalize(path: &Path) -> PathBuf {
    // copied from https://github.com/rust-lang/cargo/blob/2e4cfc2b7d43328b207879228a2ca7d427d188bb/src/cargo/util/paths.rs#L65-L90
    // both projects are MIT https://github.com/rust-lang/cargo/blob/master/LICENSE-MIT
    // for std impl progress see rfc https://github.com/rust-lang/rfcs/issues/2208
    // TODO: replace this once that lands
    uucore::fs::normalize_path(path)
}

#[cfg(not(windows))]
fn is_symlink_dir(_path: &Path) -> bool {
    false
}

#[cfg(windows)]
fn is_symlink_dir(path: &Path) -> bool {
    use std::os::windows::prelude::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY;

    path.symlink_metadata().map_or(false, |metadata| {
        metadata.file_type().is_symlink()
            && ((metadata.file_attributes() & FILE_ATTRIBUTE_DIRECTORY) != 0)
    })
}
