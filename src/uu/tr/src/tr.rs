//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) allocs bset dflag cflag sflag tflag

extern crate nom;

mod convert;
mod operation;
mod unicode_table;

use clap::{crate_version, Arg, Command};
use nom::AsBytes;
use operation::{translate_input, Sequence, SqueezeOperation, TranslateOperation};
use std::io::{stdin, stdout, BufReader, BufWriter};
use uucore::{format_usage, show};

use crate::operation::DeleteOperation;
use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::{display::Quotable, InvalidEncodingHandling};

static ABOUT: &str = "translate or delete characters";
const USAGE: &str = "{} [OPTION]... SET1 [SET2]";

mod options {
    pub const COMPLEMENT: &str = "complement";
    pub const DELETE: &str = "delete";
    pub const SQUEEZE: &str = "squeeze-repeats";
    pub const TRUNCATE_SET1: &str = "truncate-set1";
    pub const SETS: &str = "sets";
}

fn get_long_usage() -> String {
    "Translate, squeeze, and/or delete characters from standard input, \
     writing to standard output."
        .to_string()
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let after_help = get_long_usage();

    let matches = uu_app().after_help(&after_help[..]).get_matches_from(args);

    let delete_flag = matches.is_present(options::DELETE);
    let complement_flag = matches.is_present(options::COMPLEMENT);
    let squeeze_flag = matches.is_present(options::SQUEEZE);
    let truncate_set1_flag = matches.is_present(options::TRUNCATE_SET1);

    let sets = matches
        .values_of(options::SETS)
        .map(|v| {
            v.map(ToString::to_string)
                .map(|input| convert::reduce_octal_to_char(&input))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let sets_len = sets.len();

    if sets.is_empty() {
        return Err(UUsageError::new(1, "missing operand"));
    }

    if !(delete_flag || squeeze_flag) && sets_len < 2 {
        return Err(UUsageError::new(
            1,
            format!("missing operand after {}", sets[0].quote()),
        ));
    }

    if let Some(first) = sets.get(0) {
        if first.ends_with('\\') {
            show!(USimpleError::new(
                0,
                "warning: an unescaped backslash at end of string is not portable"
            ));
        }
    }

    let stdin = stdin();
    let mut locked_stdin = stdin.lock();
    let stdout = stdout();
    let locked_stdout = stdout.lock();
    let mut buffered_stdout = BufWriter::new(locked_stdout);

    let mut sets_iter = sets.iter().map(|c| c.as_str());
    let (set1, set2) = Sequence::solve_set_characters(
        sets_iter.next().unwrap_or_default(),
        sets_iter.next().unwrap_or_default(),
        truncate_set1_flag,
    )?;

    if delete_flag {
        if squeeze_flag {
            let mut delete_buffer = vec![];
            {
                let mut delete_writer = BufWriter::new(&mut delete_buffer);
                let delete_op = DeleteOperation::new(set1, complement_flag);
                translate_input(&mut locked_stdin, &mut delete_writer, delete_op);
            }
            {
                let mut squeeze_reader = BufReader::new(delete_buffer.as_bytes());
                let op = SqueezeOperation::new(set2, complement_flag);
                translate_input(&mut squeeze_reader, &mut buffered_stdout, op);
            }
        } else {
            let op = DeleteOperation::new(set1, complement_flag);
            translate_input(&mut locked_stdin, &mut buffered_stdout, op);
        }
    } else if squeeze_flag {
        if sets_len < 2 {
            let op = SqueezeOperation::new(set1, complement_flag);
            translate_input(&mut locked_stdin, &mut buffered_stdout, op);
        } else {
            let mut translate_buffer = vec![];
            {
                let mut writer = BufWriter::new(&mut translate_buffer);
                let op = TranslateOperation::new(set1, set2.clone(), complement_flag)?;
                translate_input(&mut locked_stdin, &mut writer, op);
            }
            {
                let mut reader = BufReader::new(translate_buffer.as_bytes());
                let squeeze_op = SqueezeOperation::new(set2, false);
                translate_input(&mut reader, &mut buffered_stdout, squeeze_op);
            }
        }
    } else {
        let op = TranslateOperation::new(set1, set2, complement_flag)?;
        translate_input(&mut locked_stdin, &mut buffered_stdout, op);
    }
    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .infer_long_args(true)
        .arg(
            Arg::new(options::COMPLEMENT)
                .visible_short_alias('C')
                .short('c')
                .long(options::COMPLEMENT)
                .help("use the complement of SET1"),
        )
        .arg(
            Arg::new(options::DELETE)
                .short('d')
                .long(options::DELETE)
                .help("delete characters in SET1, do not translate"),
        )
        .arg(
            Arg::new(options::SQUEEZE)
                .long(options::SQUEEZE)
                .short('s')
                .help(
                    "replace each sequence of a repeated character that is \
                     listed in the last specified SET, with a single occurrence \
                     of that character",
                ),
        )
        .arg(
            Arg::new(options::TRUNCATE_SET1)
                .long(options::TRUNCATE_SET1)
                .short('t')
                .help("first truncate SET1 to length of SET2"),
        )
        .arg(
            Arg::new(options::SETS)
                .multiple_occurrences(true)
                .takes_value(true)
                .min_values(1)
                .max_values(2),
        )
}
