// This file is part of the uutils coreutils package.
//
// (c) Jeremiah Peschka <jeremiah.peschka@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// spell-checker:ignore (ToDO) cpio svgz webm somegroup nlink rmvb xspf

// Missing features from GNU Coreutils:
//     --author
// -b, --escape
//     --block-size=SIZE
// -c
// -D, --Dired
// -f
//     --file-type
//     --format=WORD
//     --full-time
// -g
//     --group-directories-first
// -G, --no-group
//     --si
// -H, --dereference-command-line
//     --dereference-command-line-symlink-to-dir
//     --hide=PATTERN
//     --hyperlink[=WHEN]
//     --indicator-style=WORD
// -I, --ignore
// -k, --kibibytes
// -m
// -N, --literal
// -o
// -p, --indicator-style=slash
// -q, --hide-control-chars
//     --show-control-chars
// -Q, --quote-name
//     --quoting-style=WORD
//     --time=WORD
//     --time-style=TIME_STYLE
// -T, --tabsize=COLS
// -u
// -v
// -w, --width=COLS
// -x
// -X
// -Z, --context

#[cfg(unix)]
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use number_prefix::NumberPrefix;
#[cfg(unix)]
use std::collections::HashMap;
use std::fs;
use std::fs::{DirEntry, FileType, Metadata};
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
#[cfg(any(unix, target_os = "redox"))]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::cmp::Reverse;
#[cfg(not(unix))]
use std::time::{SystemTime, UNIX_EPOCH};

use term_grid::{Cell, Direction, Filling, Grid, GridOptions};
use time::{strftime, Timespec};
#[cfg(unix)]
use unicode_width::UnicodeWidthStr;
#[cfg(unix)]
use uucore::libc::{mode_t, S_ISGID, S_ISUID, S_ISVTX, S_IWOTH, S_IXGRP, S_IXOTH, S_IXUSR};

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "
 By default, ls will list the files and contents of any directories on
 the command line, expect that it will ignore files and directories
 whose names start with '.'
";

fn get_usage() -> String {
    format!("{0} [OPTION]... [FILE]...", executable!())
}

#[cfg(unix)]
static DEFAULT_COLORS: &str = "rs=0:di=01;34:ln=01;36:mh=00:pi=40;33:so=01;35:do=01;35:bd=40;33;01:cd=40;33;01:or=40;31;01:mi=00:su=37;41:sg=30;43:ca=30;41:tw=30;42:ow=34;42:st=37;44:ex=01;32:*.tar=01;31:*.tgz=01;31:*.arc=01;31:*.arj=01;31:*.taz=01;31:*.lha=01;31:*.lz4=01;31:*.lzh=01;31:*.lzma=01;31:*.tlz=01;31:*.txz=01;31:*.tzo=01;31:*.t7z=01;31:*.zip=01;31:*.z=01;31:*.Z=01;31:*.dz=01;31:*.gz=01;31:*.lrz=01;31:*.lz=01;31:*.lzo=01;31:*.xz=01;31:*.bz2=01;31:*.bz=01;31:*.tbz=01;31:*.tbz2=01;31:*.tz=01;31:*.deb=01;31:*.rpm=01;31:*.jar=01;31:*.war=01;31:*.ear=01;31:*.sar=01;31:*.rar=01;31:*.alz=01;31:*.ace=01;31:*.zoo=01;31:*.cpio=01;31:*.7z=01;31:*.rz=01;31:*.cab=01;31:*.jpg=01;35:*.jpeg=01;35:*.gif=01;35:*.bmp=01;35:*.pbm=01;35:*.pgm=01;35:*.ppm=01;35:*.tga=01;35:*.xbm=01;35:*.xpm=01;35:*.tif=01;35:*.tiff=01;35:*.png=01;35:*.svg=01;35:*.svgz=01;35:*.mng=01;35:*.pcx=01;35:*.mov=01;35:*.mpg=01;35:*.mpeg=01;35:*.m2v=01;35:*.mkv=01;35:*.webm=01;35:*.ogm=01;35:*.mp4=01;35:*.m4v=01;35:*.mp4v=01;35:*.vob=01;35:*.qt=01;35:*.nuv=01;35:*.wmv=01;35:*.asf=01;35:*.rm=01;35:*.rmvb=01;35:*.flc=01;35:*.avi=01;35:*.fli=01;35:*.flv=01;35:*.gl=01;35:*.dl=01;35:*.xcf=01;35:*.xwd=01;35:*.yuv=01;35:*.cgm=01;35:*.emf=01;35:*.ogv=01;35:*.ogx=01;35:*.aac=00;36:*.au=00;36:*.flac=00;36:*.m4a=00;36:*.mid=00;36:*.midi=00;36:*.mka=00;36:*.mp3=00;36:*.mpc=00;36:*.ogg=00;36:*.ra=00;36:*.wav=00;36:*.oga=00;36:*.opus=00;36:*.spx=00;36:*.xspf=00;36:";

#[cfg(unix)]
lazy_static! {
    static ref LS_COLORS: String =
        std::env::var("LS_COLORS").unwrap_or_else(|_| DEFAULT_COLORS.to_string());
    static ref COLOR_MAP: HashMap<&'static str, &'static str> = {
        let codes = LS_COLORS.split(':');
        let mut map = HashMap::new();
        for c in codes {
            let p: Vec<_> = c.splitn(2, '=').collect();
            if p.len() == 2 {
                map.insert(p[0], p[1]);
            }
        }
        map
    };
    static ref RESET_CODE: &'static str = COLOR_MAP.get("rs").unwrap_or(&"0");
    static ref LEFT_CODE: &'static str = COLOR_MAP.get("lc").unwrap_or(&"\x1b[");
    static ref RIGHT_CODE: &'static str = COLOR_MAP.get("rc").unwrap_or(&"m");
    static ref END_CODE: &'static str = COLOR_MAP.get("ec").unwrap_or(&"");
}

pub mod options {
    pub mod format {
        pub static ONELINE: &str = "1";
        pub static LONG: &str = "long";
        pub static COLUMNS: &str = "C";
    }
    pub mod files {
        pub static ALL: &str = "all";
        pub static ALMOST_ALL: &str = "almost-all";
    }
    pub mod sort {
        pub static SIZE: &str = "S";
        pub static TIME: &str = "t";
        pub static NONE: &str = "U";
    }
    pub mod time {
        pub static ACCESS: &str = "u";
        pub static CHANGE: &str = "c";
    }
    pub static FORMAT: &str = "format";
    pub static SORT: &str = "sort";
    pub static TIME: &str = "time";
    pub static IGNORE_BACKUPS: &str = "ignore-backups";
    pub static DIRECTORY: &str = "directory";
    pub static CLASSIFY: &str = "classify";
    pub static HUMAN_READABLE: &str = "human-readable";
    pub static INODE: &str = "inode";
    pub static DEREFERENCE: &str = "dereference";
    pub static NUMERIC_UID_GID: &str = "numeric-uid-gid";
    pub static REVERSE: &str = "reverse";
    pub static RECURSIVE: &str = "recursive";
    #[cfg(unix)]
    pub static COLOR: &str = "color";
    pub static PATHS: &str = "paths";
}

#[derive(PartialEq, Eq)]
enum Format {
    Columns,
    Long,
    OneLine,
}

enum Sort {
    None,
    Name,
    Size,
    Time,
}

enum SizeFormats {
    Bytes,
    Binary, // Powers of 1024, --human-readable
}

#[derive(PartialEq, Eq)]
enum Files {
    All,
    AlmostAll,
    Normal,
}

enum Time {
    Modification,
    Access,
    Change,
}

struct Config {
    format: Format,
    files: Files,
    sort: Sort,
    recursive: bool,
    reverse: bool,
    dereference: bool,
    classify: bool,
    ignore_backups: bool,
    size_format: SizeFormats,
    numeric_uid_gid: bool,
    directory: bool,
    time: Time,
    #[cfg(unix)]
    inode: bool,
    #[cfg(unix)]
    color: bool,
}

impl Config {
    fn from(options: clap::ArgMatches) -> Config {
        let format = if let Some(format_) = options.value_of(options::FORMAT) {
            match format_ {
                "long" | "verbose" => Format::Long,
                "single-column" => Format::OneLine,
                "columns" => Format::Columns,
                // below should never happen as clap already restricts the values.
                _ => panic!("Invalid field for --format")
            }
        } else if options.is_present(options::format::LONG) {
            Format::Long
        } else if options.is_present(options::format::ONELINE) {
            Format::OneLine
        } else if options.is_present(options::format::COLUMNS) {
            Format::Columns
        } else {
            Format::Columns
        };

        let files = if options.is_present(options::files::ALL) {
            Files::All
        } else if options.is_present(options::files::ALMOST_ALL) {
            Files::AlmostAll
        } else {
            Files::Normal
        };

        let sort = if let Some(field) = options.value_of(options::SORT) {
            match field {
                "none" => Sort::None,
                "name" => Sort::Name,
                "time" => Sort::Time,
                "size" => Sort::Size,
                // below should never happen as clap already restricts the values.
                _ => panic!("Invalid field for --sort")
            }
        } else if options.is_present(options::sort::TIME) {
            Sort::Time
        } else if options.is_present(options::sort::SIZE) {
            Sort::Size
        } else if options.is_present(options::sort::NONE) {
            Sort::None
        } else {
            Sort::Name
        };

        let time = if let Some(field) = options.value_of(options::TIME) {
            match field {
                "ctime" | "status" => Time::Change,
                "access" | "atime" | "use" => Time::Access,
                // below should never happen as clap already restricts the values.
                _ => panic!("Invalid field for --time")
            }
        } else if options.is_present(options::time::ACCESS) {
            Time::Access
        } else if options.is_present(options::time::CHANGE) {
            Time::Change
        } else {
            Time::Modification
        };

        #[cfg(unix)]
        let color = match options.value_of(options::COLOR) {
            None => options.is_present(options::COLOR),
            Some(val) => match val {
                "" | "always" | "yes" | "force" => true,
                "auto" | "tty" | "if-tty" => atty::is(atty::Stream::Stdout),
                /* "never" | "no" | "none" | */ _ => false,
            },
        };

        let size_format = if options.is_present(options::HUMAN_READABLE) {
            SizeFormats::Binary
        } else {
            SizeFormats::Bytes
        };

        Config {
            format,
            files,
            sort,
            recursive: options.is_present(options::RECURSIVE),
            reverse: options.is_present(options::REVERSE),
            dereference: options.is_present(options::DEREFERENCE),
            classify: options.is_present(options::CLASSIFY),
            ignore_backups: options.is_present(options::IGNORE_BACKUPS),
            size_format,
            numeric_uid_gid: options.is_present(options::NUMERIC_UID_GID),
            directory: options.is_present(options::DIRECTORY),
            time,
            #[cfg(unix)]
            color,
            #[cfg(unix)]
            inode: options.is_present(options::INODE),
        }
    }
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();

    let usage = get_usage();

    let mut app = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])

        // Format arguments
        .arg(
            Arg::with_name(options::FORMAT)
                .long(options::FORMAT)
                .help("Set the display format.")
                .takes_value(true)
                .possible_values(&["long", "verbose", "single-column", "columns"])
                .hide_possible_values(true)
                .require_equals(true)
                .overrides_with_all(&[
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::ONELINE,
                    options::format::LONG,
                ]),
        )
        .arg(
            Arg::with_name(options::format::COLUMNS)
                .short(options::format::COLUMNS)
                .help("Display the files in columns.")
        )
        .arg(
            Arg::with_name(options::format::ONELINE)
                .short(options::format::ONELINE)
                .help("List one file per line.")
        )
        .arg(
            Arg::with_name(options::format::LONG)
                .short("l")
                .long(options::format::LONG)
                .help("Display detailed information.")
        )

        // Time arguments
        .arg(
            Arg::with_name(options::TIME)
                .long(options::TIME)
                .help("Show time in <field>:\n\
                    \taccess time (-u): atime, access, use;\n\
                    \tchange time (-t): ctime, status.")
                .value_name("field")
                .takes_value(true)
                .possible_values(&["atime", "access", "use", "ctime", "status"])
                .hide_possible_values(true)
                .require_equals(true)
                .overrides_with_all(&[
                    options::TIME,
                    options::time::ACCESS,
                    options::time::CHANGE,
                ])
        )
        .arg(
            Arg::with_name(options::time::CHANGE)
                .short(options::time::CHANGE)
                .help("If the long listing format (e.g., -l, -o) is being used, print the status \
                change time (the ‘ctime’ in the inode) instead of the modification time. When \
                explicitly sorting by time (--sort=time or -t) or when not using a long listing \
                format, sort according to the status change time.",
        ))
        .arg(
            Arg::with_name(options::time::ACCESS)
                .short(options::time::ACCESS)
                .help("If the long listing format (e.g., -l, -o) is being used, print the status \
                access time instead of the modification time. When explicitly sorting by time \
                (--sort=time or -t) or when not using a long listing format, sort according to the \
                access time.")
        )

        // Sort arguments
        .arg(
            Arg::with_name(options::SORT)
                .long(options::SORT)
                .help("Sort by <field>: name, none (-U), time (-t) or size (-S)")
                .value_name("field")
                .takes_value(true)
                .possible_values(&["name", "none", "time", "size"])
                .require_equals(true)
                .overrides_with_all(&[
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                ])
        )
        .arg(
            Arg::with_name(options::sort::SIZE)
                .short(options::sort::SIZE)
                .help("Sort by file size, largest first."),
        )
        .arg(
            Arg::with_name(options::sort::TIME)
                .short(options::sort::TIME)
                .help("Sort by modification time (the 'mtime' in the inode), newest first."),
        )
        .arg(
            Arg::with_name(options::sort::NONE)
                .short(options::sort::NONE)
                .help("Do not sort; list the files in whatever order they are stored in the \
                directory.  This is especially useful when listing very large directories, \
                since not doing any sorting can be noticeably faster.")
        )

        // Other Flags
        .arg(
            Arg::with_name(options::files::ALL)
                .short("a")
                .long(options::files::ALL)
                .help("Do not ignore hidden files (files with names that start with '.')."),
        )
        .arg(
            Arg::with_name(options::files::ALMOST_ALL)
                .short("A")
                .long(options::files::ALMOST_ALL)
                .help(
                "In a directory, do not ignore all file names that start with '.', only ignore \
                '.' and '..'.",
            ),
        )
        .arg(
            Arg::with_name(options::IGNORE_BACKUPS)
                .short("B")
                .long(options::IGNORE_BACKUPS)
                .help("Ignore entries which end with ~."),
        )
        .arg(
            Arg::with_name(options::DIRECTORY)
                .short("d")
                .long(options::DIRECTORY)
                .help(
                    "Only list the names of directories, rather than listing directory contents. \
                This will not follow symbolic links unless one of `--dereference-command-line \
                (-H)`, `--dereference (-L)`, or `--dereference-command-line-symlink-to-dir` is \
                specified.",
                ),
        )
        .arg(
            Arg::with_name(options::CLASSIFY)
                .short("F")
                .long(options::CLASSIFY)
                .help("Append a character to each file name indicating the file type. Also, for \
                    regular files that are executable, append '*'. The file type indicators are \
                    '/' for directories, '@' for symbolic links, '|' for FIFOs, '=' for sockets, \
                    '>' for doors, and nothing for regular files.",
        ))
        .arg(
            Arg::with_name(options::HUMAN_READABLE)
                .short("h")
                .long(options::HUMAN_READABLE)
                .help("Print human readable file sizes (e.g. 1K 234M 56G)."),
        )
        .arg(
            Arg::with_name(options::INODE)
                .short("i")
                .long(options::INODE)
                .help("print the index number of each file"),
        )
        .arg(
            Arg::with_name(options::DEREFERENCE)
                .short("L")
                .long(options::DEREFERENCE)
                .help(
                    "When showing file information for a symbolic link, show information for the \
                file the link references rather than the link itself.",
                ),
        )
        .arg(
            Arg::with_name(options::NUMERIC_UID_GID)
                .short("n")
                .long(options::NUMERIC_UID_GID)
                .help("-l with numeric UIDs and GIDs."),
        )
        .arg(
            Arg::with_name(options::REVERSE)
                .short("r")
                .long(options::REVERSE)
                .help("Reverse whatever the sorting method is--e.g., list files in reverse \
                alphabetical order, youngest first, smallest first, or whatever.",
        ))
        .arg(
            Arg::with_name(options::RECURSIVE)
                .short("R")
                .long(options::RECURSIVE)
                .help("List the contents of all directories recursively."),
        )

        // Positional arguments
        .arg(Arg::with_name(options::PATHS).multiple(true).takes_value(true));
    
    #[cfg(unix)]
    {
        app = app.arg(
            Arg::with_name(options::COLOR)
                    .long(options::COLOR)
                    .help("Color output based on file type.")
                    .takes_value(true)
                    .require_equals(true)
                    .min_values(0),
        );
    }

    let matches = app.get_matches_from(args);

    let locs = matches
        .values_of(options::PATHS)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_else(|| vec![String::from(".")]);

    list(locs, Config::from(matches))
}

fn list(locs: Vec<String>, config: Config) -> i32 {
    let number_of_locs = locs.len();

    let mut files = Vec::<PathBuf>::new();
    let mut dirs = Vec::<PathBuf>::new();
    let mut has_failed = false;
    for loc in locs {
        let p = PathBuf::from(&loc);
        if !p.exists() {
            show_error!("'{}': {}", &loc, "No such file or directory");
            // We found an error, the return code of ls should not be 0
            // And no need to continue the execution
            has_failed = true;
            continue;
        }
        let mut dir = false;

        if p.is_dir() && !config.directory {
            dir = true;
            if config.format == Format::Long && !config.dereference {
                if let Ok(md) = p.symlink_metadata() {
                    if md.file_type().is_symlink() && !p.ends_with("/") {
                        dir = false;
                    }
                }
            }
        }
        if dir {
            dirs.push(p);
        } else {
            files.push(p);
        }
    }
    sort_entries(&mut files, &config);
    display_items(&files, None, &config);

    sort_entries(&mut dirs, &config);
    for dir in dirs {
        if number_of_locs > 1 {
            println!("\n{}:", dir.to_string_lossy());
        }
        enter_directory(&dir, &config);
    }
    if has_failed {
        1
    } else {
        0
    }
}

#[cfg(any(unix, target_os = "redox"))]
fn sort_entries(entries: &mut Vec<PathBuf>, config: &Config) {
    match config.sort {
        Sort::Time => entries.sort_by_key(|k| {
            Reverse(
                get_metadata(k, config)
                    .map(|md| get_time(&md, config))
                    .unwrap_or(0),
            )
        }),
        Sort::Size => entries
            .sort_by_key(|k| Reverse(get_metadata(k, config).map(|md| md.size()).unwrap_or(0))),
        // The default sort in GNU ls is case insensitive
        Sort::Name => entries.sort_by_key(|k| k.to_string_lossy().to_lowercase()),
        Sort::None => {}
    }

    if config.reverse {
        entries.reverse();
    }
}

#[cfg(windows)]
fn is_hidden(file_path: &DirEntry) -> bool {
    let metadata = fs::metadata(file_path.path()).unwrap();
    let attr = metadata.file_attributes();
    ((attr & 0x2) > 0) || file_path.file_name().to_string_lossy().starts_with('.')
}

#[cfg(unix)]
fn is_hidden(file_path: &DirEntry) -> bool {
    file_path.file_name().to_string_lossy().starts_with('.')
}

#[cfg(windows)]
fn sort_entries(entries: &mut Vec<PathBuf>, config: &Config) {
    match config.sort {
        Sort::Time => entries.sort_by_key(|k| {
            // Newest first
            Reverse(get_time(get_metadata(k, config), config).unwrap_or(std::time::UNIX_EPOCH))
        }),
        Sort::Size => entries.sort_by_key(|k| {
            // Largest first
            Reverse(
                get_metadata(k, config)
                    .map(|md| md.file_size())
                    .unwrap_or(0),
            )
        }),
        Sort::Name => entries.sort(),
        Sort::None => {}
    }

    if config.reverse {
        entries.reverse();
    }
}

fn should_display(entry: &DirEntry, config: &Config) -> bool {
    let ffi_name = entry.file_name();
    let name = ffi_name.to_string_lossy();
    if config.files == Files::Normal && is_hidden(entry) {
        return false;
    }
    if config.ignore_backups && name.ends_with('~') {
        return false;
    }
    true
}

fn enter_directory(dir: &PathBuf, config: &Config) {
    let mut entries: Vec<_> = safe_unwrap!(fs::read_dir(dir).and_then(Iterator::collect));

    entries.retain(|e| should_display(e, config));

    let mut entries: Vec<_> = entries.iter().map(DirEntry::path).collect();
    sort_entries(&mut entries, config);

    if config.files == Files::All {
        let mut display_entries = entries.clone();
        display_entries.insert(0, dir.join(".."));
        display_entries.insert(0, dir.join("."));
        display_items(&display_entries, Some(dir), config);
    } else {
        display_items(&entries, Some(dir), config);
    }

    if config.recursive {
        for e in entries.iter().filter(|p| p.is_dir()) {
            println!("\n{}:", e.to_string_lossy());
            enter_directory(&e, config);
        }
    }
}

fn get_metadata(entry: &PathBuf, config: &Config) -> std::io::Result<Metadata> {
    if config.dereference {
        entry.metadata().or_else(|_| entry.symlink_metadata())
    } else {
        entry.symlink_metadata()
    }
}

fn display_dir_entry_size(entry: &PathBuf, config: &Config) -> (usize, usize) {
    if let Ok(md) = get_metadata(entry, config) {
        (
            display_symlink_count(&md).len(),
            display_file_size(&md, config).len(),
        )
    } else {
        (0, 0)
    }
}

fn pad_left(string: String, count: usize) -> String {
    format!("{:>width$}", string, width = count)
}

fn display_items(items: &[PathBuf], strip: Option<&Path>, config: &Config) {
    if config.format == Format::Long || config.numeric_uid_gid {
        let (mut max_links, mut max_size) = (1, 1);
        for item in items {
            let (links, size) = display_dir_entry_size(item, config);
            max_links = links.max(max_links);
            max_size = size.max(max_size);
        }
        for item in items {
            display_item_long(item, strip, max_links, max_size, config);
        }
    } else {
        if config.format != Format::OneLine {
            let names = items.iter().filter_map(|i| {
                let md = get_metadata(i, config);
                match md {
                    Err(e) => {
                        let filename = get_file_name(i, strip);
                        show_error!("'{}': {}", filename, e);
                        None
                    }
                    Ok(md) => Some(display_file_name(&i, strip, &md, config)),
                }
            });

            if let Some(size) = termsize::get() {
                let mut grid = Grid::new(GridOptions {
                    filling: Filling::Spaces(2),
                    direction: Direction::TopToBottom,
                });

                for name in names {
                    grid.add(name);
                }

                if let Some(output) = grid.fit_into_width(size.cols as usize) {
                    print!("{}", output);
                    return;
                }
            }
        }

        // Couldn't display a grid, either because we don't know
        // the terminal width or because fit_into_width failed
        for i in items {
            let md = get_metadata(i, config);
            if let Ok(md) = md {
                println!("{}", display_file_name(&i, strip, &md, config).contents);
            }
        }
    }
}

use uucore::fs::display_permissions;

fn display_item_long(
    item: &PathBuf,
    strip: Option<&Path>,
    max_links: usize,
    max_size: usize,
    config: &Config,
) {
    let md = match get_metadata(item, config) {
        Err(e) => {
            let filename = get_file_name(&item, strip);
            show_error!("{}: {}", filename, e);
            return;
        }
        Ok(md) => md,
    };

    println!(
        "{}{}{} {} {} {} {} {} {}",
        get_inode(&md, config),
        display_file_type(md.file_type()),
        display_permissions(&md),
        pad_left(display_symlink_count(&md), max_links),
        display_uname(&md, config),
        display_group(&md, config),
        pad_left(display_file_size(&md, config), max_size),
        display_date(&md, config),
        display_file_name(&item, strip, &md, config).contents
    );
}

#[cfg(unix)]
fn get_inode(metadata: &Metadata, config: &Config) -> String {
    if config.inode {
        format!("{:8} ", metadata.ino())
    } else {
        "".to_string()
    }
}

#[cfg(not(unix))]
fn get_inode(_metadata: &Metadata, _config: &Config) -> String {
    "".to_string()
}

// Currently getpwuid is `linux` target only. If it's broken out into
// a posix-compliant attribute this can be updated...
#[cfg(unix)]
use uucore::entries;

#[cfg(unix)]
fn display_uname(metadata: &Metadata, config: &Config) -> String {
    if config.numeric_uid_gid {
        metadata.uid().to_string()
    } else {
        entries::uid2usr(metadata.uid()).unwrap_or_else(|_| metadata.uid().to_string())
    }
}

#[cfg(unix)]
fn display_group(metadata: &Metadata, config: &Config) -> String {
    if config.numeric_uid_gid {
        metadata.gid().to_string()
    } else {
        entries::gid2grp(metadata.gid()).unwrap_or_else(|_| metadata.gid().to_string())
    }
}

#[cfg(not(unix))]
fn display_uname(_metadata: &Metadata, _config: &Config) -> String {
    "somebody".to_string()
}

#[cfg(not(unix))]
#[allow(unused_variables)]
fn display_group(_metadata: &Metadata, _config: &Config) -> String {
    "somegroup".to_string()
}

// The implementations for get_time are separated because some options, such
// as ctime will not be available
#[cfg(unix)]
fn get_time(md: &Metadata, config: &Config) -> i64 {
    match config.time {
        Time::Change => md.ctime(),
        Time::Modification => md.mtime(),
        Time::Access => md.atime(),
    }
}

#[cfg(not(unix))]
fn get_time(md: &Metadata, config: &Config) -> Option<SystemTime> {
    match config.time {
        Time::Modification => md.modification().ok(),
        Time::Access => md.access().ok(),
        _ => None,
    }
}

#[cfg(unix)]
fn display_date(metadata: &Metadata, config: &Config) -> String {
    let secs = get_time(metadata, config);
    let time = time::at(Timespec::new(secs, 0));
    strftime("%F %R", &time).unwrap()
}

#[cfg(not(unix))]
fn display_date(metadata: &Metadata, config: &Config) -> String {
    if let Some(time) = get_time(metadata, config) {
        let time = time::at(Timespec::new(
            time.duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            0,
        ));
        strftime("%F %R", &time).unwrap()
    } else {
        "???".to_string()
    }
}

fn display_file_size(metadata: &Metadata, config: &Config) -> String {
    // NOTE: The human-readable behaviour deviates from the GNU ls.
    // The GNU ls uses binary prefixes by default.
    match config.size_format {
        SizeFormats::Binary => match NumberPrefix::decimal(metadata.len() as f64) {
            NumberPrefix::Standalone(bytes) => bytes.to_string(),
            NumberPrefix::Prefixed(prefix, bytes) => {
                format!("{:.2}{}", bytes, prefix).to_uppercase()
            }
        },
        SizeFormats::Bytes => metadata.len().to_string(),
    }
}

fn display_file_type(file_type: FileType) -> String {
    if file_type.is_dir() {
        "d".to_string()
    } else if file_type.is_symlink() {
        "l".to_string()
    } else {
        "-".to_string()
    }
}

fn get_file_name(name: &Path, strip: Option<&Path>) -> String {
    let mut name = match strip {
        Some(prefix) => name.strip_prefix(prefix).unwrap_or(name),
        None => name,
    };
    if name.as_os_str().is_empty() {
        name = Path::new(".");
    }
    name.to_string_lossy().into_owned()
}

#[cfg(not(unix))]
fn display_file_name(
    path: &Path,
    strip: Option<&Path>,
    metadata: &Metadata,
    config: &Config,
) -> Cell {
    let mut name = get_file_name(path, strip);

    if config.format == Format::Long {
        name = get_inode(metadata, config) + &name;
    }

    if config.classify {
        let file_type = metadata.file_type();
        if file_type.is_dir() {
            name.push('/');
        } else if file_type.is_symlink() {
            name.push('@');
        }
    }

    if config.format == Format::Long && metadata.file_type().is_symlink() {
        if let Ok(target) = path.read_link() {
            // We don't bother updating width here because it's not used for long listings
            let target_name = target.to_string_lossy().to_string();
            name.push_str(" -> ");
            name.push_str(&target_name);
        }
    }

    name.into()
}

#[cfg(unix)]
fn color_name(name: String, typ: &str) -> String {
    let mut typ = typ;
    if !COLOR_MAP.contains_key(typ) {
        if typ == "or" {
            typ = "ln";
        } else if typ == "mi" {
            typ = "fi";
        }
    };
    if let Some(code) = COLOR_MAP.get(typ) {
        format!(
            "{}{}{}{}{}{}{}{}",
            *LEFT_CODE, code, *RIGHT_CODE, name, *END_CODE, *LEFT_CODE, *RESET_CODE, *RIGHT_CODE,
        )
    } else {
        name
    }
}

#[cfg(unix)]
macro_rules! has {
    ($mode:expr, $perm:expr) => {
        $mode & ($perm as mode_t) != 0
    };
}

#[cfg(unix)]
#[allow(clippy::cognitive_complexity)]
fn display_file_name(
    path: &Path,
    strip: Option<&Path>,
    metadata: &Metadata,
    config: &Config,
) -> Cell {
    let mut name = get_file_name(path, strip);
    if config.format != Format::Long {
        name = get_inode(metadata, config) + &name;
    }
    let mut width = UnicodeWidthStr::width(&*name);

    let ext;

    if config.color || config.classify {
        let file_type = metadata.file_type();

        let (code, sym) = if file_type.is_dir() {
            ("di", Some('/'))
        } else if file_type.is_symlink() {
            if path.exists() {
                ("ln", Some('@'))
            } else {
                ("or", Some('@'))
            }
        } else if file_type.is_socket() {
            ("so", Some('='))
        } else if file_type.is_fifo() {
            ("pi", Some('|'))
        } else if file_type.is_block_device() {
            ("bd", None)
        } else if file_type.is_char_device() {
            ("cd", None)
        } else if file_type.is_file() {
            let mode = metadata.mode() as mode_t;
            let sym = if has!(mode, S_IXUSR | S_IXGRP | S_IXOTH) {
                Some('*')
            } else {
                None
            };
            if has!(mode, S_ISUID) {
                ("su", sym)
            } else if has!(mode, S_ISGID) {
                ("sg", sym)
            } else if has!(mode, S_ISVTX) && has!(mode, S_IWOTH) {
                ("tw", sym)
            } else if has!(mode, S_ISVTX) {
                ("st", sym)
            } else if has!(mode, S_IWOTH) {
                ("ow", sym)
            } else if has!(mode, S_IXUSR | S_IXGRP | S_IXOTH) {
                ("ex", sym)
            } else if metadata.nlink() > 1 {
                ("mh", sym)
            } else if let Some(e) = path.extension() {
                ext = format!("*.{}", e.to_string_lossy());
                (ext.as_str(), None)
            } else {
                ("fi", None)
            }
        } else {
            ("", None)
        };

        if config.color {
            name = color_name(name, code);
        }
        if config.classify {
            if let Some(s) = sym {
                name.push(s);
                width += 1;
            }
        }
    }

    if config.format == Format::Long && metadata.file_type().is_symlink() {
        if let Ok(target) = path.read_link() {
            // We don't bother updating width here because it's not used for long listings
            let code = if target.exists() { "fi" } else { "mi" };
            let target_name = color_name(target.to_string_lossy().to_string(), code);
            name.push_str(" -> ");
            name.push_str(&target_name);
        }
    }

    Cell {
        contents: name,
        width,
    }
}

#[cfg(not(unix))]
fn display_symlink_count(_metadata: &Metadata) -> String {
    // Currently not sure of how to get this on Windows, so I'm punting.
    // Git Bash looks like it may do the same thing.
    String::from("1")
}

#[cfg(unix)]
fn display_symlink_count(metadata: &Metadata) -> String {
    metadata.nlink().to_string()
}
