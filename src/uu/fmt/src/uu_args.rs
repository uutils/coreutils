// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};

use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("fmt.md");
const USAGE: &str = help_usage!("fmt.md");

pub mod options {
    pub const CROWN_MARGIN: &str = "crown-margin";
    pub const TAGGED_PARAGRAPH: &str = "tagged-paragraph";
    pub const PRESERVE_HEADERS: &str = "preserve-headers";
    pub const SPLIT_ONLY: &str = "split-only";
    pub const UNIFORM_SPACING: &str = "uniform-spacing";
    pub const PREFIX: &str = "prefix";
    pub const SKIP_PREFIX: &str = "skip-prefix";
    pub const EXACT_PREFIX: &str = "exact-prefix";
    pub const EXACT_SKIP_PREFIX: &str = "exact-skip-prefix";
    pub const WIDTH: &str = "width";
    pub const GOAL: &str = "goal";
    pub const QUICK: &str = "quick";
    pub const TAB_WIDTH: &str = "tab-width";
    pub const FILES_OR_WIDTH: &str = "files";
}

#[allow(clippy::too_many_lines)]
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::CROWN_MARGIN)
                .short('c')
                .long(options::CROWN_MARGIN)
                .help(
                    "First and second line of paragraph \
                    may have different indentations, in which \
                    case the first line's indentation is preserved, \
                    and each subsequent line's indentation matches the second line.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TAGGED_PARAGRAPH)
                .short('t')
                .long("tagged-paragraph")
                .help(
                    "Like -c, except that the first and second line of a paragraph *must* \
                    have different indentation or they are treated as separate paragraphs.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PRESERVE_HEADERS)
                .short('m')
                .long("preserve-headers")
                .help(
                    "Attempt to detect and preserve mail headers in the input. \
                    Be careful when combining this flag with -p.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SPLIT_ONLY)
                .short('s')
                .long("split-only")
                .help("Split lines only, do not reflow.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::UNIFORM_SPACING)
                .short('u')
                .long("uniform-spacing")
                .help(
                    "Insert exactly one \
                    space between words, and two between sentences. \
                    Sentence breaks in the input are detected as [?!.] \
                    followed by two spaces or a newline; other punctuation \
                    is not interpreted as a sentence break.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PREFIX)
                .short('p')
                .long("prefix")
                .help(
                    "Reformat only lines \
                    beginning with PREFIX, reattaching PREFIX to reformatted lines. \
                    Unless -x is specified, leading whitespace will be ignored \
                    when matching PREFIX.",
                )
                .value_name("PREFIX"),
        )
        .arg(
            Arg::new(options::SKIP_PREFIX)
                .short('P')
                .long("skip-prefix")
                .help(
                    "Do not reformat lines \
                    beginning with PSKIP. Unless -X is specified, leading whitespace \
                    will be ignored when matching PSKIP",
                )
                .value_name("PSKIP"),
        )
        .arg(
            Arg::new(options::EXACT_PREFIX)
                .short('x')
                .long("exact-prefix")
                .help(
                    "PREFIX must match at the \
                    beginning of the line with no preceding whitespace.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::EXACT_SKIP_PREFIX)
                .short('X')
                .long("exact-skip-prefix")
                .help(
                    "PSKIP must match at the \
                    beginning of the line with no preceding whitespace.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WIDTH)
                .short('w')
                .long("width")
                .help("Fill output lines up to a maximum of WIDTH columns, default 75. This can be specified as a negative number in the first argument.")
                // We must accept invalid values if they are overridden later. This is not supported by clap, so accept all strings instead.
                .value_name("WIDTH"),
        )
        .arg(
            Arg::new(options::GOAL)
                .short('g')
                .long("goal")
                .help("Goal width, default of 93% of WIDTH. Must be less than or equal to WIDTH.")
                // We must accept invalid values if they are overridden later. This is not supported by clap, so accept all strings instead.
                .value_name("GOAL"),
        )
        .arg(
            Arg::new(options::QUICK)
                .short('q')
                .long("quick")
                .help(
                    "Break lines more quickly at the \
            expense of a potentially more ragged appearance.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TAB_WIDTH)
                .short('T')
                .long("tab-width")
                .help(
                    "Treat tabs as TABWIDTH spaces for \
                    determining line length, default 8. Note that this is used only for \
                    calculating line lengths; tabs are preserved in the output.",
                )
                .value_name("TABWIDTH"),
        )
        .arg(
            Arg::new(options::FILES_OR_WIDTH)
                .action(ArgAction::Append)
                .value_name("FILES")
                .value_hint(clap::ValueHint::FilePath)
                .allow_negative_numbers(true),
        )
}
