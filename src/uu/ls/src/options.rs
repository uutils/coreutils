pub mod format {
    pub static ONE_LINE: &str = "1";
    pub static LONG: &str = "long";
    pub static COLUMNS: &str = "C";
    pub static ACROSS: &str = "x";
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
    pub static HUMAN_READABLE: &str = "human-readable";
    pub static SI: &str = "si";
}
pub mod quoting {
    pub static ESCAPE: &str = "escape";
    pub static LITERAL: &str = "literal";
    pub static C: &str = "quote-name";
}
pub static QUOTING_STYLE: &str = "quoting-style";
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

pub static ABOUT: &str = "
By default, ls will list the files and contents of any directories on
the command line, expect that it will ignore files and directories
whose names start with '.'
";

pub static AFTER_HELP: &str = "The TIME_STYLE argument can be full-iso, long-iso, iso.
Also the TIME_STYLE environment variable sets the default style to use.";
