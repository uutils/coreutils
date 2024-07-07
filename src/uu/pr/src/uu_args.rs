// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};

use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("pr.md");
const USAGE: &str = help_usage!("pr.md");
const AFTER_HELP: &str = help_section!("after help", "pr.md");

pub mod options {
    pub const HEADER: &str = "header";
    pub const DOUBLE_SPACE: &str = "double-space";
    pub const NUMBER_LINES: &str = "number-lines";
    pub const FIRST_LINE_NUMBER: &str = "first-line-number";
    pub const PAGES: &str = "pages";
    pub const OMIT_HEADER: &str = "omit-header";
    pub const PAGE_LENGTH: &str = "length";
    pub const NO_FILE_WARNINGS: &str = "no-file-warnings";
    pub const FORM_FEED: &str = "form-feed";
    pub const COLUMN_WIDTH: &str = "width";
    pub const PAGE_WIDTH: &str = "page-width";
    pub const ACROSS: &str = "across";
    pub const COLUMN: &str = "column";
    pub const COLUMN_CHAR_SEPARATOR: &str = "separator";
    pub const COLUMN_STRING_SEPARATOR: &str = "sep-string";
    pub const MERGE: &str = "merge";
    pub const INDENT: &str = "indent";
    pub const JOIN_LINES: &str = "join-lines";
    pub const HELP: &str = "help";
    pub const FILES: &str = "files";
}

#[allow(clippy::too_many_lines)]
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .args_override_self(true)
        .disable_help_flag(true)
        .arg(
            Arg::new(options::PAGES)
                .long(options::PAGES)
                .help("Begin and stop printing with page FIRST_PAGE[:LAST_PAGE]")
                .value_name("FIRST_PAGE[:LAST_PAGE]"),
        )
        .arg(
            Arg::new(options::HEADER)
                .short('h')
                .long(options::HEADER)
                .help(
                    "Use the string header to replace the file name \
                    in the header line.",
                )
                .value_name("STRING"),
        )
        .arg(
            Arg::new(options::DOUBLE_SPACE)
                .short('d')
                .long(options::DOUBLE_SPACE)
                .help(
                    "Produce output that is double spaced. An extra <newline> \
                character is output following every <newline> found in the input.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NUMBER_LINES)
                .short('n')
                .long(options::NUMBER_LINES)
                .help(
                    "Provide width digit line numbering.  The default for width, \
                if not specified, is 5.  The number occupies the first width column \
                positions of each text column or each line of -m output.  If char \
                (any non-digit character) is given, it is appended to the line number \
                to separate it from whatever follows.  The default for char is a <tab>. \
                Line numbers longer than width columns are truncated.",
                )
                .allow_hyphen_values(true)
                .value_name("[char][width]"),
        )
        .arg(
            Arg::new(options::FIRST_LINE_NUMBER)
                .short('N')
                .long(options::FIRST_LINE_NUMBER)
                .help("start counting with NUMBER at 1st line of first page printed")
                .value_name("NUMBER"),
        )
        .arg(
            Arg::new(options::OMIT_HEADER)
                .short('t')
                .long(options::OMIT_HEADER)
                .help(
                    "Write neither the five-line identifying header nor the five-line \
                trailer usually supplied for each page. Quit writing after the last line \
                 of each file without spacing to the end of the page.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PAGE_LENGTH)
                .short('l')
                .long(options::PAGE_LENGTH)
                .help(
                    "Override the 66-line default (default number of lines of text 56, \
                    and with -F 63) and reset the page length to lines.  If lines is not \
                    greater than the sum  of  both the  header  and trailer depths (in lines), \
                    the pr utility shall suppress both the header and trailer, as if the -t \
                    option were in effect. ",
                )
                .value_name("PAGE_LENGTH"),
        )
        .arg(
            Arg::new(options::NO_FILE_WARNINGS)
                .short('r')
                .long(options::NO_FILE_WARNINGS)
                .help("omit warning when a file cannot be opened")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FORM_FEED)
                .short('F')
                .short_alias('f')
                .long(options::FORM_FEED)
                .help(
                    "Use a <form-feed> for new pages, instead of the default behavior that \
                uses a sequence of <newline>s.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::COLUMN_WIDTH)
                .short('w')
                .long(options::COLUMN_WIDTH)
                .help(
                    "Set the width of the line to width column positions for multiple \
                text-column output only. If the -w option is not specified and the -s option \
                is not specified, the default width shall be 72. If the -w option is not specified \
                and the -s option is specified, the default width shall be 512.",
                )
                .value_name("width"),
        )
        .arg(
            Arg::new(options::PAGE_WIDTH)
                .short('W')
                .long(options::PAGE_WIDTH)
                .help(
                    "set page width to PAGE_WIDTH (72) characters always, \
                truncate lines, except -J option is set, no interference \
                with -S or -s",
                )
                .value_name("width"),
        )
        .arg(
            Arg::new(options::ACROSS)
                .short('a')
                .long(options::ACROSS)
                .help(
                    "Modify the effect of the - column option so that the columns are filled \
                across the page in a  round-robin  order (for example, when column is 2, the \
                first input line heads column 1, the second heads column 2, the third is the \
                second line in column 1, and so on).",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::COLUMN)
                .long(options::COLUMN)
                .help(
                    "Produce multi-column output that is arranged in column columns \
                (the default shall be 1) and is written down each column  in  the order in which \
                the text is received from the input file. This option should not be used with -m. \
                The options -e and -i shall be assumed for multiple text-column output.  Whether \
                or not text columns are produced with identical vertical lengths is unspecified, \
                but a text column shall never exceed the length of the page (see the -l option). \
                When used with -t, use the minimum number of lines to write the output.",
                )
                .value_name("column"),
        )
        .arg(
            Arg::new(options::COLUMN_CHAR_SEPARATOR)
                .short('s')
                .long(options::COLUMN_CHAR_SEPARATOR)
                .help(
                    "Separate text columns by the single character char instead of by the \
                appropriate number of <space>s (default for char is the <tab> character).",
                )
                .value_name("char"),
        )
        .arg(
            Arg::new(options::COLUMN_STRING_SEPARATOR)
                .short('S')
                .long(options::COLUMN_STRING_SEPARATOR)
                .help(
                    "separate columns by STRING, \
                without -S: Default separator <TAB> with -J and <space> \
                otherwise (same as -S\" \"), no effect on column options",
                )
                .value_name("string"),
        )
        .arg(
            Arg::new(options::MERGE)
                .short('m')
                .long(options::MERGE)
                .help(
                    "Merge files. Standard output shall be formatted so the pr utility \
                writes one line from each file specified by a file operand, side by side \
                into text columns of equal fixed widths, in terms of the number of column \
                positions. Implementations shall support merging of at least nine file operands.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::INDENT)
                .short('o')
                .long(options::INDENT)
                .help(
                    "Each line of output shall be preceded by offset <space>s. If the -o \
                option is not specified, the default offset shall be zero. The space taken is \
                in addition to the output line width (see the -w option below).",
                )
                .value_name("margin"),
        )
        .arg(
            Arg::new(options::JOIN_LINES)
                .short('J')
                .help(
                    "merge full lines, turns off -W line truncation, no column \
                alignment, --sep-string[=STRING] sets separators",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help("Print help information")
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(options::FILES)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
}
