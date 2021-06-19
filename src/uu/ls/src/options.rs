pub mod format {
    pub static ONE_LINE: &str = "1";
    pub static LONG: &str = "long";
    pub static COLUMNS: &str = "C";
    pub static ACROSS: &str = "x";
    pub static COMMAS: &str = "m";
    pub static LONG_NO_OWNER: &str = "g";
    pub static LONG_NO_GROUP: &str = "o";
    pub static LONG_NO_GROUP_HELP: &str =
        "Long format without group information. Identical to --format=long with --no-group.";
    pub static LONG_NUMERIC_UID_GID: &str = "numeric-uid-gid";
}

pub mod files {
    pub static ALL: &str = "all";
    pub static ALMOST_ALL: &str = "almost-all";
    pub static ALMOST_ALL_HELP: &str =
        "In a directory, do not ignore all file names that start with '.', only ignore \
    '.' and '..'.";
}

pub mod sort {
    pub static SIZE: &str = "S";
    pub static TIME: &str = "t";
    pub static NONE: &str = "U";
    pub static NONE_HELP: &str =
        "Do not sort; list the files in whatever order they are stored in the \
    directory.  This is especially useful when listing very large directories, \
    since not doing any sorting can be noticeably faster.";
    pub static VERSION: &str = "v";
    pub static EXTENSION: &str = "X";
}

pub mod time {
    pub static ACCESS: &str = "u";
    pub static ACCESS_HELP: &str =
        "If the long listing format (e.g., -l, -o) is being used, print the status \
    access time instead of the modification time. When explicitly sorting by time \
    (--sort=time or -t) or when not using a long listing format, sort according to the \
    access time.";
    pub static CHANGE: &str = "c";
    pub static CHANGE_HELP: &str =
        "If the long listing format (e.g., -l, -o) is being used, print the status \
    change time (the ‘ctime’ in the inode) instead of the modification time. When \
    explicitly sorting by time (--sort=time or -t) or when not using a long listing \
    format, sort according to the status change time.";
}

pub mod size {
    pub static HUMAN_READABLE: &str = "human-readable";
    pub static SI: &str = "si";
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
    pub static CLASSIFY_HELP: &str =
        "Append a character to each file name indicating the file type. Also, for \
    regular files that are executable, append '*'. The file type indicators are \
    '/' for directories, '@' for symbolic links, '|' for FIFOs, '=' for sockets, \
    '>' for doors, and nothing for regular files.";
}

pub mod dereference {
    pub static ALL: &str = "dereference";
    pub static ALL_HELP: &str =
        "When showing file information for a symbolic link, show information for the \
    file the link references rather than the link itself.";
    pub static ARGS: &str = "dereference-command-line";
    pub static DIR_ARGS: &str = "dereference-command-line-symlink-to-dir";
    pub static DIR_ARGS_HELP: &str =
        "Do not dereference symlinks except when they link to directories and are \
    given as command line arguments.";
}

pub static QUOTING_STYLE: &str = "quoting-style";
pub static HIDE_CONTROL_CHARS: &str = "hide-control-chars";
pub static SHOW_CONTROL_CHARS: &str = "show-control-chars";
pub static WIDTH: &str = "width";

pub static AUTHOR: &str = "author";
pub static AUTHOR_HELP: &str =
    "Show author in long format. On the supported platforms, the author \
always matches the file owner.";

pub static NO_GROUP: &str = "no-group";
pub static FORMAT: &str = "format";
pub static SORT: &str = "sort";
pub static TIME: &str = "time";
pub static IGNORE_BACKUPS: &str = "ignore-backups";

pub static DIRECTORY: &str = "directory";
pub static DIRECTORY_HELP: &str =
    "Only list the names of directories, rather than listing directory contents. \
This will not follow symbolic links unless one of `--dereference-command-line \
(-H)`, `--dereference (-L)`, or `--dereference-command-line-symlink-to-dir` is \
specified.";

pub static INODE: &str = "inode";

pub static REVERSE: &str = "reverse";
pub static REVERSE_HLEP: &str =
    "Reverse whatever the sorting method is--e.g., list files in reverse \
alphabetical order, youngest first, smallest first, or whatever.";

pub static RECURSIVE: &str = "recursive";
pub static COLOR: &str = "color";

pub static PATHS: &str = "paths";
pub static PATHS_AFTER_HELP: &str = "The TIME_STYLE argument can be full-iso, long-iso, iso.
Also the TIME_STYLE environment variable sets the default style to use.";

pub static INDICATOR_STYLE: &str = "indicator-style";
pub static INDICATOR_STYLE_HELP: &str =
    " append indicator with style WORD to entry names: none (default),  slash\
(-p), file-type (--file-type), classify (-F)";

pub static TIME_STYLE: &str = "time-style";
pub static FULL_TIME: &str = "full-time";
pub static HIDE: &str = "hide";
pub static IGNORE: &str = "ignore";

pub static ABOUT: &str = "
 By default, ls will list the files and contents of any directories on
 the command line, expect that it will ignore files and directories
 whose names start with '.'
";
