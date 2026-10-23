// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore clocale tabsize dired uerror

//! The runtime schema also provides option spellings and value counts for help,
//! manual pages, completions, and the legacy Clap configuration adapter.

use crate::{options, options::QUOTING_STYLE};
use clap::{Arg, ArgAction, builder::PossibleValue};
use uucore::parser::{gnu::OptionSpec, shortcut_value_parser::ShortcutValueParser};

pub(crate) static OPTIONS: &[OptionSpec] = &[
    OptionSpec::flag(options::HELP, &[], &[options::HELP], "ls-help-print-help"),
    OptionSpec::required(
        options::FORMAT,
        &[],
        &[options::FORMAT],
        "ls-help-set-display-format",
        "format",
    )
    .default_value_name(),
    OptionSpec::flag(
        options::format::COLUMNS,
        &['C'],
        &[],
        "ls-help-display-files-columns",
    ),
    OptionSpec::flag(
        options::format::LONG,
        &['l'],
        &[options::format::LONG],
        "ls-help-display-detailed-info",
    ),
    OptionSpec::flag(
        options::format::ACROSS,
        &['x'],
        &[],
        "ls-help-list-entries-rows",
    ),
    OptionSpec::required(
        options::format::TAB_SIZE,
        &['T'],
        &[options::format::TAB_SIZE],
        "ls-help-assume-tab-stops",
        "COLS",
    )
    .short_required_equals(),
    OptionSpec::flag(
        options::format::COMMAS,
        &['m'],
        &[],
        "ls-help-list-entries-commas",
    ),
    OptionSpec::flag(
        options::ZERO,
        &[],
        &[options::ZERO],
        "ls-help-list-entries-nul",
    ),
    OptionSpec::flag(
        options::DIRED,
        &['D'],
        &[options::DIRED],
        "ls-help-generate-dired-output",
    ),
    OptionSpec::optional(
        options::HYPERLINK,
        &[],
        &[options::HYPERLINK],
        "ls-help-hyperlink-filenames",
        "always",
        "WHEN",
    ),
    OptionSpec::flag(
        options::format::ONE_LINE,
        &['1'],
        &[],
        "ls-help-list-one-file-per-line",
    ),
    OptionSpec::flag(
        options::format::LONG_NO_GROUP,
        &['o'],
        &[],
        "ls-help-long-format-no-group",
    ),
    OptionSpec::flag(
        options::format::LONG_NO_OWNER,
        &['g'],
        &[],
        "ls-help-long-no-owner",
    ),
    OptionSpec::flag(
        options::format::LONG_NUMERIC_UID_GID,
        &['n'],
        &[options::format::LONG_NUMERIC_UID_GID],
        "ls-help-long-numeric-uid-gid",
    ),
    OptionSpec::required(
        QUOTING_STYLE,
        &[],
        &[QUOTING_STYLE],
        "ls-help-set-quoting-style",
        "quoting-style",
    )
    .default_value_name(),
    OptionSpec::flag(
        options::quoting::LITERAL,
        &['N'],
        &[options::quoting::LITERAL, "l"],
        "ls-help-literal-quoting-style",
    ),
    OptionSpec::flag(
        options::quoting::ESCAPE,
        &['b'],
        &[options::quoting::ESCAPE],
        "ls-help-escape-quoting-style",
    ),
    OptionSpec::flag(
        options::quoting::C,
        &['Q'],
        &[options::quoting::C],
        "ls-help-c-quoting-style",
    ),
    OptionSpec::flag(
        options::HIDE_CONTROL_CHARS,
        &['q'],
        &[options::HIDE_CONTROL_CHARS],
        "ls-help-replace-control-chars",
    ),
    OptionSpec::flag(
        options::SHOW_CONTROL_CHARS,
        &[],
        &[options::SHOW_CONTROL_CHARS],
        "ls-help-show-control-chars",
    ),
    OptionSpec::required(
        options::TIME,
        &[],
        &[options::TIME],
        "ls-help-show-time-field",
        "field",
    ),
    OptionSpec::flag(options::time::CHANGE, &['c'], &[], "ls-help-time-change"),
    OptionSpec::flag(options::time::ACCESS, &['u'], &[], "ls-help-time-access"),
    OptionSpec::required(
        options::HIDE,
        &[],
        &[options::HIDE],
        "ls-help-hide-pattern",
        "PATTERN",
    ),
    OptionSpec::required(
        options::IGNORE,
        &['I'],
        &[options::IGNORE],
        "ls-help-ignore-pattern",
        "PATTERN",
    )
    .short_required_equals(),
    OptionSpec::flag(
        options::IGNORE_BACKUPS,
        &['B'],
        &[options::IGNORE_BACKUPS],
        "ls-help-ignore-backups",
    ),
    OptionSpec::required(
        options::SORT,
        &[],
        &[options::SORT],
        "ls-help-sort-by-field",
        "field",
    ),
    OptionSpec::flag(options::sort::SIZE, &['S'], &[], "ls-help-sort-by-size"),
    OptionSpec::flag(options::sort::TIME, &['t'], &[], "ls-help-sort-by-time"),
    OptionSpec::flag(
        options::sort::VERSION,
        &['v'],
        &[],
        "ls-help-sort-by-version",
    ),
    OptionSpec::flag(
        options::sort::EXTENSION,
        &['X'],
        &[],
        "ls-help-sort-by-extension",
    ),
    OptionSpec::flag(options::sort::NONE, &['U'], &[], "ls-help-sort-none"),
    OptionSpec::flag(
        options::dereference::ALL,
        &['L'],
        &[options::dereference::ALL],
        "ls-help-dereference-all",
    ),
    OptionSpec::flag(
        options::dereference::DIR_ARGS,
        &[],
        &[options::dereference::DIR_ARGS],
        "ls-help-dereference-dir-args",
    ),
    OptionSpec::flag(
        options::dereference::ARGS,
        &['H'],
        &[options::dereference::ARGS],
        "ls-help-dereference-args",
    ),
    OptionSpec::flag(
        options::NO_GROUP,
        &['G'],
        &[options::NO_GROUP],
        "ls-help-no-group",
    ),
    OptionSpec::flag(options::AUTHOR, &[], &[options::AUTHOR], "ls-help-author"),
    OptionSpec::flag(
        options::files::ALL,
        &['a'],
        &[options::files::ALL],
        "ls-help-all-files",
    ),
    OptionSpec::flag(
        options::files::ALMOST_ALL,
        &['A'],
        &[options::files::ALMOST_ALL],
        "ls-help-almost-all",
    ),
    OptionSpec::flag(
        options::files::UNSORTED_ALL,
        &['f'],
        &[],
        "ls-help-unsorted-all",
    ),
    OptionSpec::flag(
        options::DIRECTORY,
        &['d'],
        &[options::DIRECTORY],
        "ls-help-directory",
    ),
    OptionSpec::flag(
        options::size::HUMAN_READABLE,
        &['h'],
        &[options::size::HUMAN_READABLE],
        "ls-help-human-readable",
    ),
    OptionSpec::flag(
        options::size::KIBIBYTES,
        &['k'],
        &[options::size::KIBIBYTES],
        "ls-help-kibibytes",
    ),
    OptionSpec::flag(options::size::SI, &[], &[options::size::SI], "ls-help-si"),
    OptionSpec::required(
        options::size::BLOCK_SIZE,
        &[],
        &[options::size::BLOCK_SIZE],
        "ls-help-block-size",
        "BLOCK_SIZE",
    ),
    OptionSpec::flag(
        options::INODE,
        &['i'],
        &[options::INODE],
        "ls-help-print-inode",
    ),
    OptionSpec::flag(
        options::REVERSE,
        &['r'],
        &[options::REVERSE],
        "ls-help-reverse-sort",
    ),
    OptionSpec::flag(
        options::RECURSIVE,
        &['R'],
        &[options::RECURSIVE],
        "ls-help-recursive",
    ),
    OptionSpec::required(
        options::WIDTH,
        &['w'],
        &[options::WIDTH],
        "ls-help-terminal-width",
        "COLS",
    )
    .short_required_equals(),
    OptionSpec::flag(
        options::size::ALLOCATION_SIZE,
        &['s'],
        &[options::size::ALLOCATION_SIZE],
        "ls-help-allocation-size",
    ),
    OptionSpec::optional(
        options::COLOR,
        &[],
        &[options::COLOR],
        "ls-help-color-output",
        "always",
        "color",
    )
    .default_value_name(),
    OptionSpec::required(
        options::INDICATOR_STYLE,
        &[],
        &[options::INDICATOR_STYLE],
        "ls-help-indicator-style",
        "indicator-style",
    )
    .default_value_name(),
    OptionSpec::optional(
        options::indicator_style::CLASSIFY,
        &['F'],
        &[options::indicator_style::CLASSIFY],
        "ls-help-classify",
        "always",
        "when",
    )
    .short_optional_equals(),
    OptionSpec::flag(
        options::indicator_style::FILE_TYPE,
        &[],
        &[options::indicator_style::FILE_TYPE],
        "ls-help-file-type",
    ),
    OptionSpec::flag(
        options::indicator_style::SLASH,
        &['p'],
        &[],
        "ls-help-slash-directories",
    ),
    OptionSpec::required(
        options::TIME_STYLE,
        &[],
        &[options::TIME_STYLE],
        "ls-help-time-style",
        "TIME_STYLE",
    ),
    OptionSpec::flag(
        options::FULL_TIME,
        &[],
        &[options::FULL_TIME],
        "ls-help-full-time",
    ),
    OptionSpec::flag(
        options::CONTEXT,
        &['Z'],
        &[options::CONTEXT],
        "ls-help-context",
    ),
    OptionSpec::flag(
        options::GROUP_DIRECTORIES_FIRST,
        &[],
        &[options::GROUP_DIRECTORIES_FIRST],
        "ls-help-group-directories-first",
    ),
    OptionSpec::flag("version", &['V'], &["version"], "").help_literal("Print version"),
];

pub(crate) fn presentation(spec: &OptionSpec) -> Arg {
    let arg = spec.arg();
    match spec.id {
        id if id == options::HELP => arg.action(ArgAction::Help),
        id if id == options::FORMAT => arg
            .value_parser(ShortcutValueParser::new(choices(spec.id).iter().map(
                |choice| PossibleValue::new(choice.name).aliases(choice.aliases.iter().copied()),
            )))
            .hide_possible_values(true)
            .overrides_with_all([
                options::FORMAT,
                options::format::COLUMNS,
                options::format::LONG,
                options::format::ACROSS,
                options::format::COLUMNS,
            ]),
        id if id == options::format::COLUMNS => arg.overrides_with_all([
            options::FORMAT,
            options::format::COLUMNS,
            options::format::LONG,
            options::format::ACROSS,
            options::format::COLUMNS,
        ]),
        id if id == options::format::LONG => arg.overrides_with_all([
            options::FORMAT,
            options::format::COLUMNS,
            options::format::LONG,
            options::format::ACROSS,
            options::format::COLUMNS,
        ]),
        id if id == options::format::ACROSS => arg.overrides_with_all([
            options::FORMAT,
            options::format::COLUMNS,
            options::format::LONG,
            options::format::ACROSS,
            options::format::COLUMNS,
        ]),
        id if id == options::format::TAB_SIZE => arg.env("TABSIZE"),
        id if id == options::format::COMMAS => arg.overrides_with_all([
            options::FORMAT,
            options::format::COLUMNS,
            options::format::LONG,
            options::format::ACROSS,
            options::format::COLUMNS,
        ]),
        id if id == options::ZERO => arg.overrides_with(options::ZERO),
        id if id == options::HYPERLINK => arg
            .value_parser(ShortcutValueParser::new(choices(spec.id).iter().map(
                |choice| PossibleValue::new(choice.name).aliases(choice.aliases.iter().copied()),
            )))
            .default_value("never"),
        id if id == QUOTING_STYLE => arg
            .value_parser(ShortcutValueParser::new(choices(spec.id).iter().map(
                |choice| PossibleValue::new(choice.name).aliases(choice.aliases.iter().copied()),
            )))
            .overrides_with_all([
                QUOTING_STYLE,
                options::quoting::LITERAL,
                options::quoting::ESCAPE,
                options::quoting::C,
            ]),
        id if id == options::quoting::LITERAL => arg.overrides_with_all([
            QUOTING_STYLE,
            options::quoting::LITERAL,
            options::quoting::ESCAPE,
            options::quoting::C,
        ]),
        id if id == options::quoting::ESCAPE => arg.overrides_with_all([
            QUOTING_STYLE,
            options::quoting::LITERAL,
            options::quoting::ESCAPE,
            options::quoting::C,
        ]),
        id if id == options::quoting::C => arg.overrides_with_all([
            QUOTING_STYLE,
            options::quoting::LITERAL,
            options::quoting::ESCAPE,
            options::quoting::C,
        ]),
        id if id == options::HIDE_CONTROL_CHARS => {
            arg.overrides_with_all([options::HIDE_CONTROL_CHARS, options::SHOW_CONTROL_CHARS])
        }
        id if id == options::SHOW_CONTROL_CHARS => {
            arg.overrides_with_all([options::HIDE_CONTROL_CHARS, options::SHOW_CONTROL_CHARS])
        }
        id if id == options::TIME => arg
            .value_parser(ShortcutValueParser::new(choices(spec.id).iter().map(
                |choice| PossibleValue::new(choice.name).aliases(choice.aliases.iter().copied()),
            )))
            .hide_possible_values(true)
            .overrides_with_all([options::TIME, options::time::ACCESS, options::time::CHANGE]),
        id if id == options::time::CHANGE => {
            arg.overrides_with_all([options::TIME, options::time::ACCESS, options::time::CHANGE])
        }
        id if id == options::time::ACCESS => {
            arg.overrides_with_all([options::TIME, options::time::ACCESS, options::time::CHANGE])
        }
        id if id == options::HIDE => arg.action(ArgAction::Append),
        id if id == options::IGNORE => arg.action(ArgAction::Append),
        id if id == options::SORT => arg
            .value_parser(ShortcutValueParser::new(choices(spec.id).iter().map(
                |choice| PossibleValue::new(choice.name).aliases(choice.aliases.iter().copied()),
            )))
            .overrides_with_all([
                options::SORT,
                options::sort::SIZE,
                options::sort::TIME,
                options::sort::NONE,
                options::sort::VERSION,
                options::sort::EXTENSION,
            ]),
        id if id == options::sort::SIZE => arg.overrides_with_all([
            options::SORT,
            options::sort::SIZE,
            options::sort::TIME,
            options::sort::NONE,
            options::sort::VERSION,
            options::sort::EXTENSION,
        ]),
        id if id == options::sort::TIME => arg.overrides_with_all([
            options::SORT,
            options::sort::SIZE,
            options::sort::TIME,
            options::sort::NONE,
            options::sort::VERSION,
            options::sort::EXTENSION,
        ]),
        id if id == options::sort::VERSION => arg.overrides_with_all([
            options::SORT,
            options::sort::SIZE,
            options::sort::TIME,
            options::sort::NONE,
            options::sort::VERSION,
            options::sort::EXTENSION,
        ]),
        id if id == options::sort::EXTENSION => arg.overrides_with_all([
            options::SORT,
            options::sort::SIZE,
            options::sort::TIME,
            options::sort::NONE,
            options::sort::VERSION,
            options::sort::EXTENSION,
        ]),
        id if id == options::sort::NONE => arg.overrides_with_all([
            options::SORT,
            options::sort::SIZE,
            options::sort::TIME,
            options::sort::NONE,
            options::sort::VERSION,
            options::sort::EXTENSION,
        ]),
        id if id == options::dereference::ALL => arg.overrides_with_all([
            options::dereference::ALL,
            options::dereference::DIR_ARGS,
            options::dereference::ARGS,
        ]),
        id if id == options::dereference::DIR_ARGS => arg.overrides_with_all([
            options::dereference::ALL,
            options::dereference::DIR_ARGS,
            options::dereference::ARGS,
        ]),
        id if id == options::dereference::ARGS => arg.overrides_with_all([
            options::dereference::ALL,
            options::dereference::DIR_ARGS,
            options::dereference::ARGS,
        ]),
        id if id == options::files::ALL => arg
            // Overrides -A (as the order matters)
            .overrides_with_all([options::files::ALL, options::files::ALMOST_ALL]),
        id if id == options::files::ALMOST_ALL => arg
            // Overrides -a (as the order matters)
            .overrides_with_all([options::files::ALL, options::files::ALMOST_ALL]),
        id if id == options::size::HUMAN_READABLE => {
            arg.overrides_with_all([options::size::BLOCK_SIZE, options::size::SI])
        }
        id if id == options::size::SI => {
            arg.overrides_with_all([options::size::BLOCK_SIZE, options::size::HUMAN_READABLE])
        }
        id if id == options::size::BLOCK_SIZE => {
            arg.overrides_with_all([options::size::SI, options::size::HUMAN_READABLE])
        }
        id if id == options::COLOR => {
            arg.value_parser(ShortcutValueParser::new(choices(spec.id).iter().map(
                |choice| PossibleValue::new(choice.name).aliases(choice.aliases.iter().copied()),
            )))
        }
        id if id == options::INDICATOR_STYLE => arg
            .value_parser(ShortcutValueParser::new(choices(spec.id).iter().map(
                |choice| PossibleValue::new(choice.name).aliases(choice.aliases.iter().copied()),
            )))
            .overrides_with_all([
                options::indicator_style::FILE_TYPE,
                options::indicator_style::SLASH,
                options::indicator_style::CLASSIFY,
                options::INDICATOR_STYLE,
            ]),
        id if id == options::indicator_style::CLASSIFY => arg
            .value_parser(ShortcutValueParser::new(choices(spec.id).iter().map(
                |choice| PossibleValue::new(choice.name).aliases(choice.aliases.iter().copied()),
            )))
            .overrides_with_all([
                options::indicator_style::FILE_TYPE,
                options::indicator_style::SLASH,
                options::indicator_style::CLASSIFY,
                options::INDICATOR_STYLE,
            ]),
        id if id == options::indicator_style::FILE_TYPE => arg.overrides_with_all([
            options::indicator_style::FILE_TYPE,
            options::indicator_style::SLASH,
            options::indicator_style::CLASSIFY,
            options::INDICATOR_STYLE,
        ]),
        id if id == options::indicator_style::SLASH => arg.overrides_with_all([
            options::indicator_style::FILE_TYPE,
            options::indicator_style::SLASH,
            options::indicator_style::CLASSIFY,
            options::INDICATOR_STYLE,
        ]),
        id if id == options::TIME_STYLE => arg
            .env("TIME_STYLE")
            .overrides_with_all([options::TIME_STYLE]),
        id if id == options::FULL_TIME => arg.overrides_with(options::FULL_TIME),
        "version" => arg.action(ArgAction::Version),
        _ => arg,
    }
}

/// Accepted values are shared by the runtime resolver and Clap's tooling
/// projection. Aliases in one row have the same meaning, including prefixes.
struct Choice {
    name: &'static str,
    aliases: &'static [&'static str],
}

const WHEN: &[Choice] = &[
    Choice {
        name: "always",
        aliases: &["yes", "force"],
    },
    Choice {
        name: "auto",
        aliases: &["tty", "if-tty"],
    },
    Choice {
        name: "never",
        aliases: &["no", "none"],
    },
];
const FORMATS: &[Choice] = &[
    Choice {
        name: "long",
        aliases: &[],
    },
    Choice {
        name: "verbose",
        aliases: &[],
    },
    Choice {
        name: "single-column",
        aliases: &[],
    },
    Choice {
        name: "columns",
        aliases: &[],
    },
    Choice {
        name: "vertical",
        aliases: &[],
    },
    Choice {
        name: "across",
        aliases: &[],
    },
    Choice {
        name: "horizontal",
        aliases: &[],
    },
    Choice {
        name: "commas",
        aliases: &[],
    },
];
const TIMES: &[Choice] = &[
    Choice {
        name: "atime",
        aliases: &["access", "use"],
    },
    Choice {
        name: "ctime",
        aliases: &["status"],
    },
    Choice {
        name: "mtime",
        aliases: &["modification"],
    },
    Choice {
        name: "birth",
        aliases: &["creation"],
    },
];
const SORTS: &[Choice] = &[
    Choice {
        name: "name",
        aliases: &[],
    },
    Choice {
        name: "none",
        aliases: &[],
    },
    Choice {
        name: "time",
        aliases: &[],
    },
    Choice {
        name: "size",
        aliases: &[],
    },
    Choice {
        name: "version",
        aliases: &[],
    },
    Choice {
        name: "extension",
        aliases: &[],
    },
    Choice {
        name: "width",
        aliases: &[],
    },
];
const INDICATORS: &[Choice] = &[
    Choice {
        name: "none",
        aliases: &[],
    },
    Choice {
        name: "slash",
        aliases: &[],
    },
    Choice {
        name: "file-type",
        aliases: &[],
    },
    Choice {
        name: "classify",
        aliases: &[],
    },
];
const QUOTING: &[Choice] = &[
    Choice {
        name: "literal",
        aliases: &[],
    },
    Choice {
        name: "locale",
        aliases: &[],
    },
    Choice {
        name: "shell",
        aliases: &[],
    },
    Choice {
        name: "shell-escape",
        aliases: &[],
    },
    Choice {
        name: "shell-always",
        aliases: &[],
    },
    Choice {
        name: "shell-escape-always",
        aliases: &[],
    },
    Choice {
        name: "clocale",
        aliases: &[],
    },
    Choice {
        name: "c",
        aliases: &[],
    },
    Choice {
        name: "c-maybe",
        aliases: &[],
    },
    Choice {
        name: "escape",
        aliases: &[],
    },
];

fn choices(id: &str) -> &'static [Choice] {
    if [
        options::COLOR,
        options::HYPERLINK,
        options::indicator_style::CLASSIFY,
    ]
    .contains(&id)
    {
        WHEN
    } else if id == options::FORMAT {
        FORMATS
    } else if id == options::TIME {
        TIMES
    } else if id == options::SORT {
        SORTS
    } else if id == options::INDICATOR_STYLE {
        INDICATORS
    } else if id == QUOTING_STYLE {
        QUOTING
    } else {
        &[]
    }
}

pub(crate) fn resolve_value(
    option: &uucore::parser::gnu::Occurrence,
    diag_args: Option<&[std::ffi::OsString]>,
) -> uucore::error::UResult<&'static str> {
    let choices = choices(option.id);
    if choices.is_empty() {
        return Ok("");
    }
    let input = option.value.as_deref().and_then(std::ffi::OsStr::to_str);
    if let Some(input) = input {
        if let Some(choice) = choices
            .iter()
            .find(|choice| choice.name == input || choice.aliases.contains(&input))
        {
            return Ok(choice.name);
        }
        let mut candidates = choices.iter().filter(|choice| {
            std::iter::once(choice.name)
                .chain(choice.aliases.iter().copied())
                .any(|name| name.starts_with(input))
        });
        if let Some(choice) = candidates.next()
            && candidates.next().is_none()
        {
            return Ok(choice.name);
        }
    }
    let valid: Vec<&str> = choices.iter().map(|choice| choice.name).collect();
    Err(
        uucore::parser::gnu::ParseError::invalid_value(option, &valid)
            .to_uerror_with_args(1, diag_args),
    )
}
