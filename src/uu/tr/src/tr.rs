// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) allocs bset dflag cflag sflag tflag

mod operation;
mod unicode_table;

use clap::{crate_version, Arg, ArgAction, Command};
use nom::AsBytes;
use operation::{translate_input, Sequence, SqueezeOperation, TranslateOperation};
use std::io::{stdin, stdout, BufReader, BufWriter};
use uucore::{format_usage, help_about, help_section, help_usage, show};

use crate::operation::DeleteOperation;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, UUsageError};

const ABOUT: &str = help_about!("tr.md");
const AFTER_HELP: &str = help_section!("after help", "tr.md");
const USAGE: &str = help_usage!("tr.md");

mod options {
    pub const COMPLEMENT: &str = "complement";
    pub const DELETE: &str = "delete";
    pub const SQUEEZE: &str = "squeeze-repeats";
    pub const TRUNCATE_SET1: &str = "truncate-set1";
    pub const SETS: &str = "sets";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().after_help(AFTER_HELP).try_get_matches_from(args)?;

    let delete_flag = matches.get_flag(options::DELETE);
    let complement_flag = matches.get_flag(options::COMPLEMENT);
    let squeeze_flag = matches.get_flag(options::SQUEEZE);
    let truncate_set1_flag = matches.get_flag(options::TRUNCATE_SET1);

    // Ultimately this should be OsString, but we might want to wait for the
    // pattern API on OsStr
    let sets: Vec<_> = matches
        .get_many::<String>(options::SETS)
        .into_iter()
        .flatten()
        .map(ToOwned::to_owned)
        .collect();

    let sets_len = sets.len();

    if sets.is_empty() {
        return Err(UUsageError::new(1, "missing operand"));
    }

    if !(delete_flag || squeeze_flag) && sets_len < 2 {
        return Err(UUsageError::new(
            1,
            format!(
                "missing operand after {}\nTwo strings must be given when translating.",
                sets[0].quote()
            ),
        ));
    }

    if sets_len > 1 {
        let start = "extra operand";
        if delete_flag && !squeeze_flag {
            let op = sets[1].quote();
            let msg = if sets_len == 2 {
                format!(
                    "{} {}\nOnly one string may be given when deleting without squeezing repeats.",
                    start, op,
                )
            } else {
                format!("{} {}", start, op,)
            };
            return Err(UUsageError::new(1, msg));
        }
        if sets_len > 2 {
            let op = sets[2].quote();
            let msg = format!("{} {}", start, op);
            return Err(UUsageError::new(1, msg));
        }
    }

    if let Some(first) = sets.first() {
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
        sets_iter.next().unwrap_or_default().as_bytes(),
        sets_iter.next().unwrap_or_default().as_bytes(),
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

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::COMPLEMENT)
                .visible_short_alias('C')
                .short('c')
                .long(options::COMPLEMENT)
                .help("use the complement of SET1")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DELETE)
                .short('d')
                .long(options::DELETE)
                .help("delete characters in SET1, do not translate")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SQUEEZE)
                .long(options::SQUEEZE)
                .short('s')
                .help(
                    "replace each sequence of a repeated character that is \
                     listed in the last specified SET, with a single occurrence \
                     of that character",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TRUNCATE_SET1)
                .long(options::TRUNCATE_SET1)
                .short('t')
                .help("first truncate SET1 to length of SET2")
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new(options::SETS).num_args(1..))
}
