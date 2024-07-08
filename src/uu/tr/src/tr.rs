// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) allocs bset dflag cflag sflag tflag

use crate::operation::{
    translate_input, Sequence, SqueezeOperation, SymbolTranslator, TranslateOperation,
};
use std::io::{stdin, stdout, BufWriter};
use uucore::show;

use crate::operation::DeleteOperation;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, UUsageError};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = crate::uu_app().try_get_matches_from(args)?;

    let delete_flag = matches.get_flag(crate::options::DELETE);
    let complement_flag = matches.get_flag(crate::options::COMPLEMENT);
    let squeeze_flag = matches.get_flag(crate::options::SQUEEZE);
    let truncate_set1_flag = matches.get_flag(crate::options::TRUNCATE_SET1);

    // Ultimately this should be OsString, but we might want to wait for the
    // pattern API on OsStr
    let sets: Vec<_> = matches
        .get_many::<String>(crate::options::SETS)
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

    // According to the man page: translating only happens if deleting or if a second set is given
    let translating = !delete_flag && sets.len() > 1;
    let mut sets_iter = sets.iter().map(|c| c.as_str());
    let (set1, set2) = Sequence::solve_set_characters(
        sets_iter.next().unwrap_or_default().as_bytes(),
        sets_iter.next().unwrap_or_default().as_bytes(),
        complement_flag,
        // if we are not translating then we don't truncate set1
        truncate_set1_flag && translating,
        translating,
    )?;

    // '*_op' are the operations that need to be applied, in order.
    if delete_flag {
        if squeeze_flag {
            let delete_op = DeleteOperation::new(set1);
            let squeeze_op = SqueezeOperation::new(set2);
            let op = delete_op.chain(squeeze_op);
            translate_input(&mut locked_stdin, &mut buffered_stdout, op);
        } else {
            let op = DeleteOperation::new(set1);
            translate_input(&mut locked_stdin, &mut buffered_stdout, op);
        }
    } else if squeeze_flag {
        if sets_len < 2 {
            let op = SqueezeOperation::new(set1);
            translate_input(&mut locked_stdin, &mut buffered_stdout, op);
        } else {
            let translate_op = TranslateOperation::new(set1, set2.clone())?;
            let squeeze_op = SqueezeOperation::new(set2);
            let op = translate_op.chain(squeeze_op);
            translate_input(&mut locked_stdin, &mut buffered_stdout, op);
        }
    } else {
        let op = TranslateOperation::new(set1, set2)?;
        translate_input(&mut locked_stdin, &mut buffered_stdout, op);
    }
    Ok(())
}
