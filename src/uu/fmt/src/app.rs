use clap::{crate_version, App, Arg};

// spell-checker:ignore (ToDO) PSKIP tabwidth

const ABOUT: &str = "Reformat paragraphs from input files (or stdin) to stdout.";

pub const OPT_CROWN_MARGIN: &str = "crown-margin";
pub const OPT_TAGGED_PARAGRAPH: &str = "tagged-paragraph";
pub const OPT_PRESERVE_HEADERS: &str = "preserve-headers";
pub const OPT_SPLIT_ONLY: &str = "split-only";
pub const OPT_UNIFORM_SPACING: &str = "uniform-spacing";
pub const OPT_PREFIX: &str = "prefix";
pub const OPT_SKIP_PREFIX: &str = "skip-prefix";
pub const OPT_EXACT_PREFIX: &str = "exact-prefix";
pub const OPT_EXACT_SKIP_PREFIX: &str = "exact-skip-prefix";
pub const OPT_WIDTH: &str = "width";
pub const OPT_GOAL: &str = "goal";
pub const OPT_QUICK: &str = "quick";
pub const OPT_TAB_WIDTH: &str = "tab-width";

pub const ARG_FILES: &str = "files";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(OPT_CROWN_MARGIN)
                .short("c")
                .long(OPT_CROWN_MARGIN)
                .help(
                    "First and second line of paragraph
    may have different indentations, in which
    case the first line's indentation is preserved,
    and each subsequent line's indentation matches the second line.",
                ),
        )
        .arg(
            Arg::with_name(OPT_TAGGED_PARAGRAPH)
                .short("t")
                .long("tagged-paragraph")
                .help(
                    "Like -c, except that the first and second line of a paragraph *must*
                have different indentation or they are treated as separate paragraphs.",
                ),
        )
        .arg(
            Arg::with_name(OPT_PRESERVE_HEADERS)
                .short("m")
                .long("preserve-headers")
                .help(
                    "Attempt to detect and preserve mail headers in the input.
                Be careful when combining this flag with -p.",
                ),
        )
        .arg(
            Arg::with_name(OPT_SPLIT_ONLY)
                .short("s")
                .long("split-only")
                .help("Split lines only, do not reflow."),
        )
        .arg(
            Arg::with_name(OPT_UNIFORM_SPACING)
                .short("u")
                .long("uniform-spacing")
                .help(
                    "Insert exactly one
                space between words, and two between sentences.
                Sentence breaks in the input are detected as [?!.]
                followed by two spaces or a newline; other punctuation
                is not interpreted as a sentence break.",
                ),
        )
        .arg(
            Arg::with_name(OPT_PREFIX)
                .short("p")
                .long("prefix")
                .help(
                    "Reformat only lines
                beginning with PREFIX, reattaching PREFIX to reformatted lines.
                Unless -x is specified, leading whitespace will be ignored
                when matching PREFIX.",
                )
                .value_name("PREFIX"),
        )
        .arg(
            Arg::with_name(OPT_SKIP_PREFIX)
                .short("P")
                .long("skip-prefix")
                .help(
                    "Do not reformat lines
                beginning with PSKIP. Unless -X is specified, leading whitespace
                will be ignored when matching PSKIP",
                )
                .value_name("PSKIP"),
        )
        .arg(
            Arg::with_name(OPT_EXACT_PREFIX)
                .short("x")
                .long("exact-prefix")
                .help(
                    "PREFIX must match at the
                beginning of the line with no preceding whitespace.",
                ),
        )
        .arg(
            Arg::with_name(OPT_EXACT_SKIP_PREFIX)
                .short("X")
                .long("exact-skip-prefix")
                .help(
                    "PSKIP must match at the
                beginning of the line with no preceding whitespace.",
                ),
        )
        .arg(
            Arg::with_name(OPT_WIDTH)
                .short("w")
                .long("width")
                .help("Fill output lines up to a maximum of WIDTH columns, default 79.")
                .value_name("WIDTH"),
        )
        .arg(
            Arg::with_name(OPT_GOAL)
                .short("g")
                .long("goal")
                .help("Goal width, default ~0.94*WIDTH. Must be less than WIDTH.")
                .value_name("GOAL"),
        )
        .arg(Arg::with_name(OPT_QUICK).short("q").long("quick").help(
            "Break lines more quickly at the
        expense of a potentially more ragged appearance.",
        ))
        .arg(
            Arg::with_name(OPT_TAB_WIDTH)
                .short("T")
                .long("tab-width")
                .help(
                    "Treat tabs as TABWIDTH spaces for
                determining line length, default 8. Note that this is used only for
                calculating line lengths; tabs are preserved in the output.",
                )
                .value_name("TABWIDTH"),
        )
        .arg(Arg::with_name(ARG_FILES).multiple(true).takes_value(true))
}
