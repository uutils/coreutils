// This file is part of the uutils coreutils package.
//
// (c) Jeremiah Peschka <jeremiah.peschka@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// spell-checker:ignore (ToDO) cpio svgz webm somegroup nlink rmvb xspf

#[macro_use]
extern crate uucore;
#[macro_use]
extern crate lazy_static;

mod list;
mod options;
mod quoting_style;
mod version_cmp;

use clap::{crate_version, App, Arg};

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

#[cfg(unix)]
use uucore::InvalidEncodingHandling;

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let usage = list::get_usage();

    let app = App::new(executable!())
        .version(crate_version!())
        .about(options::ABOUT)
        .usage(&usage[..])

        // Format arguments
        .arg(
            Arg::with_name(options::FORMAT)
                .long(options::FORMAT)
                .help("Set the display format.")
                .takes_value(true)
                .possible_values(&["long", "verbose", "single-column", "columns", "vertical", "across", "horizontal", "commas"])
                .hide_possible_values(true)
                .require_equals(true)
                .overrides_with_all(&[
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ]),
        )
        .arg(
            Arg::with_name(options::format::COLUMNS)
                .short(options::format::COLUMNS)
                .help("Display the files in columns.")
                .overrides_with_all(&[
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ]),
        )
        .arg(
            Arg::with_name(options::format::LONG)
                .short("l")
                .long(options::format::LONG)
                .help("Display detailed information.")
                .overrides_with_all(&[
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ]),
        )
        .arg(
            Arg::with_name(options::format::ACROSS)
                .short(options::format::ACROSS)
                .help("List entries in rows instead of in columns.")
                .overrides_with_all(&[
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ]),
        )
        .arg(
            Arg::with_name(options::format::COMMAS)
                .short(options::format::COMMAS)
                .help("List entries separated by commas.")
                .overrides_with_all(&[
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ]),
        )
        // The next four arguments do not override with the other format
        // options, see the comment in Config::from for the reason.
        // Ideally, they would use Arg::override_with, with their own name
        // but that doesn't seem to work in all cases. Example:
        // ls -1g1
        // even though `ls -11` and `ls -1 -g -1` work.
        .arg(
            Arg::with_name(options::format::ONE_LINE)
                .short(options::format::ONE_LINE)
                .help("List one file per line.")
                .multiple(true)
        )
        .arg(
            Arg::with_name(options::format::LONG_NO_GROUP)
                .short(options::format::LONG_NO_GROUP)
                .help("Long format without group information. Identical to --format=long with --no-group.")
                .multiple(true)
        )
        .arg(
            Arg::with_name(options::format::LONG_NO_OWNER)
                .short(options::format::LONG_NO_OWNER)
                .help("Long format without owner information.")
                .multiple(true)
        )
        .arg(
            Arg::with_name(options::format::LONG_NUMERIC_UID_GID)
                .short("n")
                .long(options::format::LONG_NUMERIC_UID_GID)
                .help("-l with numeric UIDs and GIDs.")
                .multiple(true)
        )

        // Quoting style
        .arg(
            Arg::with_name(options::QUOTING_STYLE)
                .long(options::QUOTING_STYLE)
                .takes_value(true)
                .help("Set quoting style.")
                .possible_values(&["literal", "shell", "shell-always", "shell-escape", "shell-escape-always", "c", "escape"])
                .overrides_with_all(&[
                    options::QUOTING_STYLE,
                    options::quoting::LITERAL,
                    options::quoting::ESCAPE,
                    options::quoting::C,
                ])
        )
        .arg(
            Arg::with_name(options::quoting::LITERAL)
                .short("N")
                .long(options::quoting::LITERAL)
                .help("Use literal quoting style. Equivalent to `--quoting-style=literal`")
                .overrides_with_all(&[
                    options::QUOTING_STYLE,
                    options::quoting::LITERAL,
                    options::quoting::ESCAPE,
                    options::quoting::C,
                ])
        )
        .arg(
            Arg::with_name(options::quoting::ESCAPE)
                .short("b")
                .long(options::quoting::ESCAPE)
                .help("Use escape quoting style. Equivalent to `--quoting-style=escape`")
                .overrides_with_all(&[
                    options::QUOTING_STYLE,
                    options::quoting::LITERAL,
                    options::quoting::ESCAPE,
                    options::quoting::C,
                ])
        )
        .arg(
            Arg::with_name(options::quoting::C)
                .short("Q")
                .long(options::quoting::C)
                .help("Use C quoting style. Equivalent to `--quoting-style=c`")
                .overrides_with_all(&[
                    options::QUOTING_STYLE,
                    options::quoting::LITERAL,
                    options::quoting::ESCAPE,
                    options::quoting::C,
                ])
        )

        // Control characters
        .arg(
            Arg::with_name(options::HIDE_CONTROL_CHARS)
                .short("q")
                .long(options::HIDE_CONTROL_CHARS)
                .help("Replace control characters with '?' if they are not escaped.")
                .overrides_with_all(&[
                    options::HIDE_CONTROL_CHARS,
                    options::SHOW_CONTROL_CHARS,
                ])
        )
        .arg(
            Arg::with_name(options::SHOW_CONTROL_CHARS)
                .long(options::SHOW_CONTROL_CHARS)
                .help("Show control characters 'as is' if they are not escaped.")
                .overrides_with_all(&[
                    options::HIDE_CONTROL_CHARS,
                    options::SHOW_CONTROL_CHARS,
                ])
        )

        // Time arguments
        .arg(
            Arg::with_name(options::TIME)
                .long(options::TIME)
                .help("Show time in <field>:\n\
                    \taccess time (-u): atime, access, use;\n\
                    \tchange time (-t): ctime, status.\n\
                    \tbirth time: birth, creation;")
                .value_name("field")
                .takes_value(true)
                .possible_values(&["atime", "access", "use", "ctime", "status", "birth", "creation"])
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
                format, sort according to the status change time.")
                .overrides_with_all(&[
                    options::TIME,
                    options::time::ACCESS,
                    options::time::CHANGE,
                ])
        )
        .arg(
            Arg::with_name(options::time::ACCESS)
                .short(options::time::ACCESS)
                .help("If the long listing format (e.g., -l, -o) is being used, print the status \
                access time instead of the modification time. When explicitly sorting by time \
                (--sort=time or -t) or when not using a long listing format, sort according to the \
                access time.")
                .overrides_with_all(&[
                    options::TIME,
                    options::time::ACCESS,
                    options::time::CHANGE,
                ])
        )

        // Hide and ignore
        .arg(
            Arg::with_name(options::HIDE)
                .long(options::HIDE)
                .takes_value(true)
                .multiple(true)
                .value_name("PATTERN")
                .help("do not list implied entries matching shell PATTERN (overridden by -a or -A)")
        )
        .arg(
            Arg::with_name(options::IGNORE)
                .short("I")
                .long(options::IGNORE)
                .takes_value(true)
                .multiple(true)
                .value_name("PATTERN")
                .help("do not list implied entries matching shell PATTERN")
        )
        .arg(
            Arg::with_name(options::IGNORE_BACKUPS)
                .short("B")
                .long(options::IGNORE_BACKUPS)
                .help("Ignore entries which end with ~."),
        )

        // Sort arguments
        .arg(
            Arg::with_name(options::SORT)
                .long(options::SORT)
                .help("Sort by <field>: name, none (-U), time (-t), size (-S) or extension (-X)")
                .value_name("field")
                .takes_value(true)
                .possible_values(&["name", "none", "time", "size", "version", "extension"])
                .require_equals(true)
                .overrides_with_all(&[
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ])
        )
        .arg(
            Arg::with_name(options::sort::SIZE)
                .short(options::sort::SIZE)
                .help("Sort by file size, largest first.")
                .overrides_with_all(&[
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ])
        )
        .arg(
            Arg::with_name(options::sort::TIME)
                .short(options::sort::TIME)
                .help("Sort by modification time (the 'mtime' in the inode), newest first.")
                .overrides_with_all(&[
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ])
        )
        .arg(
            Arg::with_name(options::sort::VERSION)
                .short(options::sort::VERSION)
                .help("Natural sort of (version) numbers in the filenames.")
                .overrides_with_all(&[
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ])
        )
        .arg(
            Arg::with_name(options::sort::EXTENSION)
                .short(options::sort::EXTENSION)
                .help("Sort alphabetically by entry extension.")
                .overrides_with_all(&[
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ])
        )
        .arg(
            Arg::with_name(options::sort::NONE)
                .short(options::sort::NONE)
                .help("Do not sort; list the files in whatever order they are stored in the \
                directory.  This is especially useful when listing very large directories, \
                since not doing any sorting can be noticeably faster.")
                .overrides_with_all(&[
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ])
        )

        // Dereferencing
        .arg(
            Arg::with_name(options::dereference::ALL)
                .short("L")
                .long(options::dereference::ALL)
                .help(
                    "When showing file information for a symbolic link, show information for the \
                file the link references rather than the link itself.",
                )
                .overrides_with_all(&[
                    options::dereference::ALL,
                    options::dereference::DIR_ARGS,
                    options::dereference::ARGS,
                ])
        )
        .arg(
            Arg::with_name(options::dereference::DIR_ARGS)
                .long(options::dereference::DIR_ARGS)
                .help(
                    "Do not dereference symlinks except when they link to directories and are \
                    given as command line arguments.",
                )
                .overrides_with_all(&[
                    options::dereference::ALL,
                    options::dereference::DIR_ARGS,
                    options::dereference::ARGS,
                ])
        )
        .arg(
            Arg::with_name(options::dereference::ARGS)
                .short("H")
                .long(options::dereference::ARGS)
                .help(
                    "Do not dereference symlinks except when given as command line arguments.",
                )
                .overrides_with_all(&[
                    options::dereference::ALL,
                    options::dereference::DIR_ARGS,
                    options::dereference::ARGS,
                ])
        )

        // Long format options
        .arg(
            Arg::with_name(options::NO_GROUP)
                .long(options::NO_GROUP)
                .short("-G")
                .help("Do not show group in long format.")
        )
        .arg(
            Arg::with_name(options::AUTHOR)
                .long(options::AUTHOR)
                .help("Show author in long format. On the supported platforms, the author \
                always matches the file owner.")
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
            Arg::with_name(options::size::HUMAN_READABLE)
                .short("h")
                .long(options::size::HUMAN_READABLE)
                .help("Print human readable file sizes (e.g. 1K 234M 56G).")
                .overrides_with(options::size::SI),
        )
        .arg(
            Arg::with_name(options::size::SI)
                .long(options::size::SI)
                .help("Print human readable file sizes using powers of 1000 instead of 1024.")
        )
        .arg(
            Arg::with_name(options::INODE)
                .short("i")
                .long(options::INODE)
                .help("print the index number of each file"),
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
        .arg(
            Arg::with_name(options::WIDTH)
                .long(options::WIDTH)
                .short("w")
                .help("Assume that the terminal is COLS columns wide.")
                .value_name("COLS")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(options::COLOR)
                .long(options::COLOR)
                .help("Color output based on file type.")
                .takes_value(true)
                .require_equals(true)
                .min_values(0),
        )
        .arg(
            Arg::with_name(options::INDICATOR_STYLE)
                .long(options::INDICATOR_STYLE)
                .help(" append indicator with style WORD to entry names: none (default),  slash\
                       (-p), file-type (--file-type), classify (-F)")
                .takes_value(true)
                .possible_values(&["none", "slash", "file-type", "classify"])
                .overrides_with_all(&[
                    options::indicator_style::FILE_TYPE,
                    options::indicator_style::SLASH,
                    options::indicator_style::CLASSIFY,
                    options::INDICATOR_STYLE,
                ]))
                .arg(
            Arg::with_name(options::indicator_style::CLASSIFY)
                .short("F")
                .long(options::indicator_style::CLASSIFY)
                .help("Append a character to each file name indicating the file type. Also, for \
                       regular files that are executable, append '*'. The file type indicators are \
                       '/' for directories, '@' for symbolic links, '|' for FIFOs, '=' for sockets, \
                       '>' for doors, and nothing for regular files.")
                .overrides_with_all(&[
                    options::indicator_style::FILE_TYPE,
                    options::indicator_style::SLASH,
                    options::indicator_style::CLASSIFY,
                    options::INDICATOR_STYLE,
                ])
        )
        .arg(
            Arg::with_name(options::indicator_style::FILE_TYPE)
                .long(options::indicator_style::FILE_TYPE)
                .help("Same as --classify, but do not append '*'")
                .overrides_with_all(&[
                    options::indicator_style::FILE_TYPE,
                    options::indicator_style::SLASH,
                    options::indicator_style::CLASSIFY,
                    options::INDICATOR_STYLE,
                ]))
        .arg(
            Arg::with_name(options::indicator_style::SLASH)
                .short(options::indicator_style::SLASH)
                .help("Append / indicator to directories."
                )
                .overrides_with_all(&[
                    options::indicator_style::FILE_TYPE,
                    options::indicator_style::SLASH,
                    options::indicator_style::CLASSIFY,
                    options::INDICATOR_STYLE,
                ]))
        .arg(
            //This still needs support for posix-*, +FORMAT
            Arg::with_name(options::TIME_STYLE)
                .long(options::TIME_STYLE)
                .help("time/date format with -l; see TIME_STYLE below")
                .value_name("TIME_STYLE")
                .env("TIME_STYLE")
                .possible_values(&[
                    "full-iso",
                    "long-iso",
                    "iso",
                    "locale",
                ])
                .overrides_with_all(&[
                    options::TIME_STYLE
                ])
        )
        .arg(
            Arg::with_name(options::FULL_TIME)
            .long(options::FULL_TIME)
            .overrides_with(options::FULL_TIME)
            .help("like -l --time-style=full-iso")
        )

    // Positional arguments
        .arg(Arg::with_name(options::PATHS).multiple(true).takes_value(true))

        .after_help(options::AFTER_HELP);

    let matches = app.get_matches_from(args);

    let locs = matches
        .values_of(options::PATHS)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_else(|| vec![String::from(".")]);

    list::list(locs, list::Config::from_matches(matches))
}
