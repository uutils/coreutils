// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
#![allow(dead_code)] //TODO

// spell-checker:ignore (change!) each's
// spell-checker:ignore (ToDO) LONGHELP FORMATSTRING templating parameterizing formatstr

use std::io::stdout;
use std::ops::ControlFlow;

use uucore::error::{UResult, UUsageError};
use uucore::format::{parse_spec_and_escape, FormatArgument, FormatItem};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = crate::uu_app().get_matches_from(args);

    let format_string = matches
        .get_one::<String>(crate::options::FORMATSTRING)
        .ok_or_else(|| UUsageError::new(1, "missing operand"))?;

    let values: Vec<_> = match matches.get_many::<String>(crate::options::ARGUMENT) {
        Some(s) => s.map(|s| FormatArgument::Unparsed(s.to_string())).collect(),
        None => vec![],
    };

    let mut format_seen = false;
    let mut args = values.iter().peekable();
    for item in parse_spec_and_escape(format_string.as_ref()) {
        if let Ok(FormatItem::Spec(_)) = item {
            format_seen = true;
        }
        match item?.write(stdout(), &mut args)? {
            ControlFlow::Continue(()) => {}
            ControlFlow::Break(()) => return Ok(()),
        };
    }

    // Without format specs in the string, the iter would not consume any args,
    // leading to an infinite loop. Thus, we exit early.
    if !format_seen {
        return Ok(());
    }

    while args.peek().is_some() {
        for item in parse_spec_and_escape(format_string.as_ref()) {
            match item?.write(stdout(), &mut args)? {
                ControlFlow::Continue(()) => {}
                ControlFlow::Break(()) => return Ok(()),
            };
        }
    }
    Ok(())
}
