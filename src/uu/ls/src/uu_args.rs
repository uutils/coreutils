// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{
    builder::{NonEmptyStringValueParser, PossibleValue, ValueParser},
    crate_version, Arg, ArgAction, Command,
};
use uucore::{format_usage, shortcut_value_parser::ShortcutValueParser};
use uucore::{help_about, help_section, help_usage};

#[cfg(not(feature = "selinux"))]
static CONTEXT_HELP_TEXT: &str = "print any security context of each file (not enabled)";
#[cfg(feature = "selinux")]
static CONTEXT_HELP_TEXT: &str = "print any security context of each file";

const ABOUT: &str = help_about!("ls.md");
const AFTER_HELP: &str = help_section!("after help", "ls.md");
const USAGE: &str = help_usage!("ls.md");

pub mod options {
    pub mod format {
        pub static ONE_LINE: &str = "1";
        pub static LONG: &str = "long";
        pub static COLUMNS: &str = "C";
        pub static ACROSS: &str = "x";
        pub static TAB_SIZE: &str = "tabsize"; // silently ignored (see #3624)
        pub static COMMAS: &str = "m";
        pub static LONG_NO_OWNER: &str = "g";
        pub static LONG_NO_GROUP: &str = "o";
        pub static LONG_NUMERIC_UID_GID: &str = "numeric-uid-gid";
    }

    pub mod files {
        pub static ALL: &str = "all";
        pub static ALMOST_ALL: &str = "almost-all";
    }

    pub mod sort {
        pub static SIZE: &str = "S";
        pub static TIME: &str = "t";
        pub static NONE: &str = "U";
        pub static VERSION: &str = "v";
        pub static EXTENSION: &str = "X";
    }

    pub mod time {
        pub static ACCESS: &str = "u";
        pub static CHANGE: &str = "c";
    }

    pub mod size {
        pub static ALLOCATION_SIZE: &str = "size";
        pub static BLOCK_SIZE: &str = "block-size";
        pub static HUMAN_READABLE: &str = "human-readable";
        pub static SI: &str = "si";
        pub static KIBIBYTES: &str = "kibibytes";
    }

    pub mod quoting {
        pub static ESCAPE: &str = "escape";
        pub static LITERAL: &str = "literal";
        pub static C: &str = "quote-name";
    }

    pub mod indicator_style {
        pub static SLASH: &str = "p";
        pub static FILE_TYPE: &str = "file-type";
        pub static CLASSIFY: &str = "classify";
    }

    pub mod dereference {
        pub static ALL: &str = "dereference";
        pub static ARGS: &str = "dereference-command-line";
        pub static DIR_ARGS: &str = "dereference-command-line-symlink-to-dir";
    }

    pub static HELP: &str = "help";
    pub static QUOTING_STYLE: &str = "quoting-style";
    pub static HIDE_CONTROL_CHARS: &str = "hide-control-chars";
    pub static SHOW_CONTROL_CHARS: &str = "show-control-chars";
    pub static WIDTH: &str = "width";
    pub static AUTHOR: &str = "author";
    pub static NO_GROUP: &str = "no-group";
    pub static FORMAT: &str = "format";
    pub static SORT: &str = "sort";
    pub static TIME: &str = "time";
    pub static IGNORE_BACKUPS: &str = "ignore-backups";
    pub static DIRECTORY: &str = "directory";
    pub static INODE: &str = "inode";
    pub static REVERSE: &str = "reverse";
    pub static RECURSIVE: &str = "recursive";
    pub static COLOR: &str = "color";
    pub static PATHS: &str = "paths";
    pub static INDICATOR_STYLE: &str = "indicator-style";
    pub static TIME_STYLE: &str = "time-style";
    pub static FULL_TIME: &str = "full-time";
    pub static HIDE: &str = "hide";
    pub static IGNORE: &str = "ignore";
    pub static CONTEXT: &str = "context";
    pub static GROUP_DIRECTORIES_FIRST: &str = "group-directories-first";
    pub static ZERO: &str = "zero";
    pub static DIRED: &str = "dired";
    pub static HYPERLINK: &str = "hyperlink";
}

#[allow(clippy::too_many_lines)]
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(ABOUT)
        .infer_long_args(true)
        .disable_help_flag(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help("Print help information.")
                .action(ArgAction::Help),
        )
        // Format arguments
        .arg(
            Arg::new(options::FORMAT)
                .long(options::FORMAT)
                .help("Set the display format.")
                .value_parser(ShortcutValueParser::new([
                    "long",
                    "verbose",
                    "single-column",
                    "columns",
                    "vertical",
                    "across",
                    "horizontal",
                    "commas",
                ]))
                .hide_possible_values(true)
                .require_equals(true)
                .overrides_with_all([
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                    options::DIRED,
                ]),
        )
        .arg(
            Arg::new(options::format::COLUMNS)
                .short('C')
                .help("Display the files in columns.")
                .overrides_with_all([
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::format::LONG)
                .short('l')
                .long(options::format::LONG)
                .help("Display detailed information.")
                .overrides_with_all([
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::format::ACROSS)
                .short('x')
                .help("List entries in rows instead of in columns.")
                .overrides_with_all([
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            // silently ignored (see #3624)
            Arg::new(options::format::TAB_SIZE)
                .short('T')
                .long(options::format::TAB_SIZE)
                .env("TABSIZE")
                .value_name("COLS")
                .help("Assume tab stops at each COLS instead of 8 (unimplemented)"),
        )
        .arg(
            Arg::new(options::format::COMMAS)
                .short('m')
                .help("List entries separated by commas.")
                .overrides_with_all([
                    options::FORMAT,
                    options::format::COLUMNS,
                    options::format::LONG,
                    options::format::ACROSS,
                    options::format::COLUMNS,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ZERO)
                .long(options::ZERO)
                .overrides_with(options::ZERO)
                .help("List entries separated by ASCII NUL characters.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DIRED)
                .long(options::DIRED)
                .short('D')
                .help("generate output designed for Emacs' dired (Directory Editor) mode")
                .action(ArgAction::SetTrue)
                .overrides_with(options::HYPERLINK),
        )
        .arg(
            Arg::new(options::HYPERLINK)
                .long(options::HYPERLINK)
                .help("hyperlink file names WHEN")
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("always").alias("yes").alias("force"),
                    PossibleValue::new("auto").alias("tty").alias("if-tty"),
                    PossibleValue::new("never").alias("no").alias("none"),
                ]))
                .require_equals(true)
                .num_args(0..=1)
                .default_missing_value("always")
                .default_value("never")
                .value_name("WHEN")
                .overrides_with(options::DIRED),
        )
        // The next four arguments do not override with the other format
        // options, see the comment in Config::from for the reason.
        // Ideally, they would use Arg::override_with, with their own name
        // but that doesn't seem to work in all cases. Example:
        // ls -1g1
        // even though `ls -11` and `ls -1 -g -1` work.
        .arg(
            Arg::new(options::format::ONE_LINE)
                .short('1')
                .help("List one file per line.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::format::LONG_NO_GROUP)
                .short('o')
                .help(
                    "Long format without group information. \
                        Identical to --format=long with --no-group.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::format::LONG_NO_OWNER)
                .short('g')
                .help("Long format without owner information.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::format::LONG_NUMERIC_UID_GID)
                .short('n')
                .long(options::format::LONG_NUMERIC_UID_GID)
                .help("-l with numeric UIDs and GIDs.")
                .action(ArgAction::SetTrue),
        )
        // Quoting style
        .arg(
            Arg::new(options::QUOTING_STYLE)
                .long(options::QUOTING_STYLE)
                .help("Set quoting style.")
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("literal"),
                    PossibleValue::new("shell"),
                    PossibleValue::new("shell-escape"),
                    PossibleValue::new("shell-always"),
                    PossibleValue::new("shell-escape-always"),
                    PossibleValue::new("c").alias("c-maybe"),
                    PossibleValue::new("escape"),
                ]))
                .overrides_with_all([
                    options::QUOTING_STYLE,
                    options::quoting::LITERAL,
                    options::quoting::ESCAPE,
                    options::quoting::C,
                ]),
        )
        .arg(
            Arg::new(options::quoting::LITERAL)
                .short('N')
                .long(options::quoting::LITERAL)
                .alias("l")
                .help("Use literal quoting style. Equivalent to `--quoting-style=literal`")
                .overrides_with_all([
                    options::QUOTING_STYLE,
                    options::quoting::LITERAL,
                    options::quoting::ESCAPE,
                    options::quoting::C,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::quoting::ESCAPE)
                .short('b')
                .long(options::quoting::ESCAPE)
                .help("Use escape quoting style. Equivalent to `--quoting-style=escape`")
                .overrides_with_all([
                    options::QUOTING_STYLE,
                    options::quoting::LITERAL,
                    options::quoting::ESCAPE,
                    options::quoting::C,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::quoting::C)
                .short('Q')
                .long(options::quoting::C)
                .help("Use C quoting style. Equivalent to `--quoting-style=c`")
                .overrides_with_all([
                    options::QUOTING_STYLE,
                    options::quoting::LITERAL,
                    options::quoting::ESCAPE,
                    options::quoting::C,
                ])
                .action(ArgAction::SetTrue),
        )
        // Control characters
        .arg(
            Arg::new(options::HIDE_CONTROL_CHARS)
                .short('q')
                .long(options::HIDE_CONTROL_CHARS)
                .help("Replace control characters with '?' if they are not escaped.")
                .overrides_with_all([options::HIDE_CONTROL_CHARS, options::SHOW_CONTROL_CHARS])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_CONTROL_CHARS)
                .long(options::SHOW_CONTROL_CHARS)
                .help("Show control characters 'as is' if they are not escaped.")
                .overrides_with_all([options::HIDE_CONTROL_CHARS, options::SHOW_CONTROL_CHARS])
                .action(ArgAction::SetTrue),
        )
        // Time arguments
        .arg(
            Arg::new(options::TIME)
                .long(options::TIME)
                .help(
                    "Show time in <field>:\n\
                        \taccess time (-u): atime, access, use;\n\
                        \tchange time (-t): ctime, status.\n\
                        \tbirth time: birth, creation;",
                )
                .value_name("field")
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("atime").alias("access").alias("use"),
                    PossibleValue::new("ctime").alias("status"),
                    PossibleValue::new("birth").alias("creation"),
                ]))
                .hide_possible_values(true)
                .require_equals(true)
                .overrides_with_all([options::TIME, options::time::ACCESS, options::time::CHANGE]),
        )
        .arg(
            Arg::new(options::time::CHANGE)
                .short('c')
                .help(
                    "If the long listing format (e.g., -l, -o) is being used, print the \
                        status change time (the 'ctime' in the inode) instead of the modification \
                        time. When explicitly sorting by time (--sort=time or -t) or when not \
                        using a long listing format, sort according to the status change time.",
                )
                .overrides_with_all([options::TIME, options::time::ACCESS, options::time::CHANGE])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::time::ACCESS)
                .short('u')
                .help(
                    "If the long listing format (e.g., -l, -o) is being used, print the \
                        status access time instead of the modification time. When explicitly \
                        sorting by time (--sort=time or -t) or when not using a long listing \
                        format, sort according to the access time.",
                )
                .overrides_with_all([options::TIME, options::time::ACCESS, options::time::CHANGE])
                .action(ArgAction::SetTrue),
        )
        // Hide and ignore
        .arg(
            Arg::new(options::HIDE)
                .long(options::HIDE)
                .action(ArgAction::Append)
                .value_name("PATTERN")
                .help(
                    "do not list implied entries matching shell PATTERN (overridden by -a or -A)",
                ),
        )
        .arg(
            Arg::new(options::IGNORE)
                .short('I')
                .long(options::IGNORE)
                .action(ArgAction::Append)
                .value_name("PATTERN")
                .help("do not list implied entries matching shell PATTERN"),
        )
        .arg(
            Arg::new(options::IGNORE_BACKUPS)
                .short('B')
                .long(options::IGNORE_BACKUPS)
                .help("Ignore entries which end with ~.")
                .action(ArgAction::SetTrue),
        )
        // Sort arguments
        .arg(
            Arg::new(options::SORT)
                .long(options::SORT)
                .help("Sort by <field>: name, none (-U), time (-t), size (-S), extension (-X) or width")
                .value_name("field")
                .value_parser(ShortcutValueParser::new(["name", "none", "time", "size", "version", "extension", "width"]))
                .require_equals(true)
                .overrides_with_all([
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ]),
        )
        .arg(
            Arg::new(options::sort::SIZE)
                .short('S')
                .help("Sort by file size, largest first.")
                .overrides_with_all([
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sort::TIME)
                .short('t')
                .help("Sort by modification time (the 'mtime' in the inode), newest first.")
                .overrides_with_all([
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sort::VERSION)
                .short('v')
                .help("Natural sort of (version) numbers in the filenames.")
                .overrides_with_all([
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sort::EXTENSION)
                .short('X')
                .help("Sort alphabetically by entry extension.")
                .overrides_with_all([
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::sort::NONE)
                .short('U')
                .help(
                    "Do not sort; list the files in whatever order they are stored in the \
                    directory.  This is especially useful when listing very large directories, \
                    since not doing any sorting can be noticeably faster.",
                )
                .overrides_with_all([
                    options::SORT,
                    options::sort::SIZE,
                    options::sort::TIME,
                    options::sort::NONE,
                    options::sort::VERSION,
                    options::sort::EXTENSION,
                ])
                .action(ArgAction::SetTrue),
        )
        // Dereferencing
        .arg(
            Arg::new(options::dereference::ALL)
                .short('L')
                .long(options::dereference::ALL)
                .help(
                    "When showing file information for a symbolic link, show information for the \
                    file the link references rather than the link itself.",
                )
                .overrides_with_all([
                    options::dereference::ALL,
                    options::dereference::DIR_ARGS,
                    options::dereference::ARGS,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::dereference::DIR_ARGS)
                .long(options::dereference::DIR_ARGS)
                .help(
                    "Do not follow symlinks except when they link to directories and are \
                    given as command line arguments.",
                )
                .overrides_with_all([
                    options::dereference::ALL,
                    options::dereference::DIR_ARGS,
                    options::dereference::ARGS,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::dereference::ARGS)
                .short('H')
                .long(options::dereference::ARGS)
                .help("Do not follow symlinks except when given as command line arguments.")
                .overrides_with_all([
                    options::dereference::ALL,
                    options::dereference::DIR_ARGS,
                    options::dereference::ARGS,
                ])
                .action(ArgAction::SetTrue),
        )
        // Long format options
        .arg(
            Arg::new(options::NO_GROUP)
                .long(options::NO_GROUP)
                .short('G')
                .help("Do not show group in long format.")
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new(options::AUTHOR).long(options::AUTHOR).help(
            "Show author in long format. On the supported platforms, \
            the author always matches the file owner.",
        ).action(ArgAction::SetTrue))
        // Other Flags
        .arg(
            Arg::new(options::files::ALL)
                .short('a')
                .long(options::files::ALL)
                // Overrides -A (as the order matters)
                .overrides_with_all([options::files::ALL, options::files::ALMOST_ALL])
                .help("Do not ignore hidden files (files with names that start with '.').")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::files::ALMOST_ALL)
                .short('A')
                .long(options::files::ALMOST_ALL)
                // Overrides -a (as the order matters)
                .overrides_with_all([options::files::ALL, options::files::ALMOST_ALL])
                .help(
                    "In a directory, do not ignore all file names that start with '.', \
                    only ignore '.' and '..'.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DIRECTORY)
                .short('d')
                .long(options::DIRECTORY)
                .help(
                    "Only list the names of directories, rather than listing directory contents. \
                    This will not follow symbolic links unless one of `--dereference-command-line \
                    (-H)`, `--dereference (-L)`, or `--dereference-command-line-symlink-to-dir` is \
                    specified.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::size::HUMAN_READABLE)
                .short('h')
                .long(options::size::HUMAN_READABLE)
                .help("Print human readable file sizes (e.g. 1K 234M 56G).")
                .overrides_with_all([options::size::BLOCK_SIZE, options::size::SI])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::size::KIBIBYTES)
                .short('k')
                .long(options::size::KIBIBYTES)
                .help(
                    "default to 1024-byte blocks for file system usage; used only with -s and per \
                    directory totals",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::size::SI)
                .long(options::size::SI)
                .help("Print human readable file sizes using powers of 1000 instead of 1024.")
                .overrides_with_all([options::size::BLOCK_SIZE, options::size::HUMAN_READABLE])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::size::BLOCK_SIZE)
                .long(options::size::BLOCK_SIZE)
                .require_equals(true)
                .value_name("BLOCK_SIZE")
                .help("scale sizes by BLOCK_SIZE when printing them")
                .overrides_with_all([options::size::SI, options::size::HUMAN_READABLE]),
        )
        .arg(
            Arg::new(options::INODE)
                .short('i')
                .long(options::INODE)
                .help("print the index number of each file")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REVERSE)
                .short('r')
                .long(options::REVERSE)
                .help(
                    "Reverse whatever the sorting method is e.g., list files in reverse \
            alphabetical order, youngest first, smallest first, or whatever.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::RECURSIVE)
                .short('R')
                .long(options::RECURSIVE)
                .help("List the contents of all directories recursively.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WIDTH)
                .long(options::WIDTH)
                .short('w')
                .help("Assume that the terminal is COLS columns wide.")
                .value_name("COLS"),
        )
        .arg(
            Arg::new(options::size::ALLOCATION_SIZE)
                .short('s')
                .long(options::size::ALLOCATION_SIZE)
                .help("print the allocated size of each file, in blocks")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::COLOR)
                .long(options::COLOR)
                .help("Color output based on file type.")
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("always").alias("yes").alias("force"),
                    PossibleValue::new("auto").alias("tty").alias("if-tty"),
                    PossibleValue::new("never").alias("no").alias("none"),
                ]))
                .require_equals(true)
                .num_args(0..=1),
        )
        .arg(
            Arg::new(options::INDICATOR_STYLE)
                .long(options::INDICATOR_STYLE)
                .help(
                    "Append indicator with style WORD to entry names: \
                none (default),  slash (-p), file-type (--file-type), classify (-F)",
                )
                .value_parser(ShortcutValueParser::new(["none", "slash", "file-type", "classify"]))
                .overrides_with_all([
                    options::indicator_style::FILE_TYPE,
                    options::indicator_style::SLASH,
                    options::indicator_style::CLASSIFY,
                    options::INDICATOR_STYLE,
                ]),
        )
        .arg(
            // The --classify flag can take an optional when argument to
            // control its behavior from version 9 of GNU coreutils.
            // There is currently an inconsistency where GNU coreutils allows only
            // the long form of the flag to take the argument while we allow it
            // for both the long and short form of the flag.
            Arg::new(options::indicator_style::CLASSIFY)
                .short('F')
                .long(options::indicator_style::CLASSIFY)
                .help(
                    "Append a character to each file name indicating the file type. Also, for \
                    regular files that are executable, append '*'. The file type indicators are \
                    '/' for directories, '@' for symbolic links, '|' for FIFOs, '=' for sockets, \
                    '>' for doors, and nothing for regular files. when may be omitted, or one of:\n\
                        \tnone - Do not classify. This is the default.\n\
                        \tauto - Only classify if standard output is a terminal.\n\
                        \talways - Always classify.\n\
                    Specifying --classify and no when is equivalent to --classify=always. This will \
                    not follow symbolic links listed on the command line unless the \
                    --dereference-command-line (-H), --dereference (-L), or \
                    --dereference-command-line-symlink-to-dir options are specified.",
                )
                .value_name("when")
                .value_parser(ShortcutValueParser::new([
                    PossibleValue::new("always").alias("yes").alias("force"),
                    PossibleValue::new("auto").alias("tty").alias("if-tty"),
                    PossibleValue::new("never").alias("no").alias("none"),
                ]))
                .default_missing_value("always")
                .require_equals(true)
                .num_args(0..=1)
                .overrides_with_all([
                    options::indicator_style::FILE_TYPE,
                    options::indicator_style::SLASH,
                    options::indicator_style::CLASSIFY,
                    options::INDICATOR_STYLE,
                ]),
        )
        .arg(
            Arg::new(options::indicator_style::FILE_TYPE)
                .long(options::indicator_style::FILE_TYPE)
                .help("Same as --classify, but do not append '*'")
                .overrides_with_all([
                    options::indicator_style::FILE_TYPE,
                    options::indicator_style::SLASH,
                    options::indicator_style::CLASSIFY,
                    options::INDICATOR_STYLE,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::indicator_style::SLASH)
                .short('p')
                .help("Append / indicator to directories.")
                .overrides_with_all([
                    options::indicator_style::FILE_TYPE,
                    options::indicator_style::SLASH,
                    options::indicator_style::CLASSIFY,
                    options::INDICATOR_STYLE,
                ])
                .action(ArgAction::SetTrue),
        )
        .arg(
            //This still needs support for posix-*
            Arg::new(options::TIME_STYLE)
                .long(options::TIME_STYLE)
                .help("time/date format with -l; see TIME_STYLE below")
                .value_name("TIME_STYLE")
                .env("TIME_STYLE")
                .value_parser(NonEmptyStringValueParser::new())
                .overrides_with_all([options::TIME_STYLE]),
        )
        .arg(
            Arg::new(options::FULL_TIME)
                .long(options::FULL_TIME)
                .overrides_with(options::FULL_TIME)
                .help("like -l --time-style=full-iso")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CONTEXT)
                .short('Z')
                .long(options::CONTEXT)
                .help(CONTEXT_HELP_TEXT)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::GROUP_DIRECTORIES_FIRST)
                .long(options::GROUP_DIRECTORIES_FIRST)
                .help(
                    "group directories before files; can be augmented with \
                    a --sort option, but any use of --sort=none (-U) disables grouping",
                )
                .action(ArgAction::SetTrue),
        )
        // Positional arguments
        .arg(
            Arg::new(options::PATHS)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath)
                .value_parser(ValueParser::os_string()),
        )
        .after_help(AFTER_HELP)
}
