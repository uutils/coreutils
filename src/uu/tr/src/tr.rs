// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) allocs bset dflag cflag sflag tflag

mod operation;
mod unicode_table;

use crate::operation::DeleteOperation;
use clap::{Arg, ArgAction, Command, value_parser};
use operation::{
    Sequence, SqueezeOperation, SymbolTranslator, TranslateOperation, translate_input,
};
use std::ffi::OsString;
use std::io::{BufWriter, Write, stdin, stdout};
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::fs::is_stdin_directory;
use uucore::{format_usage, os_str_as_bytes, show};

use uucore::locale::get_message;

mod options {
    pub const COMPLEMENT: &str = "complement";
    pub const DELETE: &str = "delete";
    pub const SQUEEZE: &str = "squeeze-repeats";
    pub const TRUNCATE_SET1: &str = "truncate-set1";
    pub const SETS: &str = "sets";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app()
        .after_help(get_message("tr-after-help"))
        .try_get_matches_from(args)?;

    let delete_flag = matches.get_flag(options::DELETE);
    let complement_flag = matches.get_flag(options::COMPLEMENT);
    let squeeze_flag = matches.get_flag(options::SQUEEZE);
    let truncate_set1_flag = matches.get_flag(options::TRUNCATE_SET1);

    // Ultimately this should be OsString, but we might want to wait for the
    // pattern API on OsStr
    let sets: Vec<_> = matches
        .get_many::<OsString>(options::SETS)
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

    if delete_flag & squeeze_flag && sets_len < 2 {
        return Err(UUsageError::new(
            1,
            format!(
                "missing operand after {}\nTwo strings must be given when deleting and squeezing.",
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
                    "{start} {op}\nOnly one string may be given when deleting without squeezing repeats.",
                )
            } else {
                format!("{start} {op}")
            };
            return Err(UUsageError::new(1, msg));
        }
        if sets_len > 2 {
            let op = sets[2].quote();
            let msg = format!("{start} {op}");
            return Err(UUsageError::new(1, msg));
        }
    }

    if let Some(first) = sets.first() {
        let slice = os_str_as_bytes(first)?;
        let trailing_backslashes = slice.iter().rev().take_while(|&&c| c == b'\\').count();
        if trailing_backslashes % 2 == 1 {
            // The trailing backslash has a non-backslash character before it.
            show!(USimpleError::new(
                0,
                "warning: an unescaped backslash at end of string is not portable"
            ));
        }
    }

    let stdin = stdin();
    let mut locked_stdin = stdin.lock();
    let mut buffered_stdout = BufWriter::new(stdout().lock());

    // According to the man page: translating only happens if deleting or if a second set is given
    let translating = !delete_flag && sets.len() > 1;
    let mut sets_iter = sets.iter().map(|c| c.as_os_str());
    let (set1, set2) = Sequence::solve_set_characters(
        os_str_as_bytes(sets_iter.next().unwrap_or_default())?,
        os_str_as_bytes(sets_iter.next().unwrap_or_default())?,
        complement_flag,
        // if we are not translating then we don't truncate set1
        truncate_set1_flag && translating,
        translating,
    )?;

    if is_stdin_directory(&stdin) {
        return Err(USimpleError::new(1, "read error: Is a directory"));
    }

    // '*_op' are the operations that need to be applied, in order.
    if delete_flag {
        if squeeze_flag {
            let delete_op = DeleteOperation::new(set1);
            let squeeze_op = SqueezeOperation::new(set2);
            let op = delete_op.chain(squeeze_op);
            translate_input(&mut locked_stdin, &mut buffered_stdout, op)?;
        } else {
            let op = DeleteOperation::new(set1);
            translate_input(&mut locked_stdin, &mut buffered_stdout, op)?;
        }
    } else if squeeze_flag {
        if sets_len < 2 {
            let op = SqueezeOperation::new(set1);
            translate_input(&mut locked_stdin, &mut buffered_stdout, op)?;
        } else {
            let translate_op = TranslateOperation::new(set1, set2.clone())?;
            let squeeze_op = SqueezeOperation::new(set2);
            let op = translate_op.chain(squeeze_op);
            translate_input(&mut locked_stdin, &mut buffered_stdout, op)?;
        }
    } else {
        let op = TranslateOperation::new(set1, set2)?;
        translate_input(&mut locked_stdin, &mut buffered_stdout, op)?;
    }

    buffered_stdout
        .flush()
        .map_err_context(|| "write error".into())?;

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(get_message("tr-about"))
        .override_usage(format_usage(&get_message("tr-usage")))
        .infer_long_args(true)
        .trailing_var_arg(true)
        .arg(
            Arg::new(options::COMPLEMENT)
                .visible_short_alias('C')
                .short('c')
                .long(options::COMPLEMENT)
                .help("use the complement of SET1")
                .action(ArgAction::SetTrue)
                .overrides_with(options::COMPLEMENT),
        )
        .arg(
            Arg::new(options::DELETE)
                .short('d')
                .long(options::DELETE)
                .help("delete characters in SET1, do not translate")
                .action(ArgAction::SetTrue)
                .overrides_with(options::DELETE),
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
                .action(ArgAction::SetTrue)
                .overrides_with(options::SQUEEZE),
        )
        .arg(
            Arg::new(options::TRUNCATE_SET1)
                .long(options::TRUNCATE_SET1)
                .short('t')
                .help("first truncate SET1 to length of SET2")
                .action(ArgAction::SetTrue)
                .overrides_with(options::TRUNCATE_SET1),
        )
        .arg(
            Arg::new(options::SETS)
                .num_args(1..)
                .value_parser(value_parser!(OsString)),
        )
}
