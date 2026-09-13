// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (libs) interner

mod error;
mod graph;
mod interner;
mod parser;

use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult, USimpleError};
use uucore::{format_usage, translate};

use crate::error::{AllocationError, Error, ReadError};
use crate::graph::GraphBuilder;
use crate::interner::Sym;

mod options {
    pub const FILE: &str = "file";
}

#[inline]
fn try_clone_str(value: &str) -> Result<String, AllocationError> {
    let mut result = String::new();
    result.try_reserve(value.len())?;
    result.push_str(value);
    Ok(result)
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let mut inputs = matches
        .get_many::<OsString>(options::FILE)
        .into_iter()
        .flatten();

    let input = inputs.next().expect("default value should be set by clap");

    if let Some(extra) = inputs.next() {
        return Err(USimpleError::new(
            1,
            translate!(
                "tsort-error-extra-operand",
                "operand" => extra.quote()
            ),
        ));
    }

    // Create the directed graph from pairs of tokens in the input data.
    let input_name = input.to_string_lossy();
    let graph_name = try_clone_str(&input_name).map_err(Error::from)?;
    let mut g = GraphBuilder::new(graph_name);

    if input == "-" {
        process_input(io::stdin().lock(), &mut g)?;
    } else {
        // some platforms cannot catch this as read error. Needs additional cost by stat
        #[cfg(windows)]
        {
            let input = std::path::Path::new(input);
            if input.is_dir() {
                let name = try_clone_str(&input.to_string_lossy()).map_err(Error::from)?;
                return Err(Error::Read(ReadError::IsDir(name)).into());
            }
        }

        let file = File::open(input).map_err_context(|| input.maybe_quote().to_string())?;

        // advise the OS we will access the data sequentially if possible
        #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
        let _ = rustix::fs::fadvise(&file, 0, None, rustix::fs::Advice::Sequential);

        let reader = BufReader::new(file);
        process_input(reader, &mut g)?;
    }

    let mut g = g.finish();
    g.run_tsort()?;
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new("tsort")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("tsort"))
        .override_usage(format_usage(&translate!("tsort-usage")))
        .about(translate!("tsort-about"))
        .infer_long_args(true)
        // no-op flag, needed for POSIX compatibility.
        .arg(
            Arg::new("warn")
                .short('w')
                .action(ArgAction::SetTrue)
                .hide(true),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .value_parser(clap::value_parser!(OsString))
                .value_hint(clap::ValueHint::FilePath)
                .default_value("-")
                .num_args(1..)
                .action(ArgAction::Append),
        )
}

impl UError for Error {}

fn process_input<R: BufRead>(reader: R, graph: &mut GraphBuilder) -> Result<(), Error> {
    let mut pending: Option<Sym> = None;

    // Input is considered to be in the format
    // From1 To1 From2 To2 ...
    // with tokens separated by whitespaces (<SPACE>, \t, or \n).
    //
    // Tokens are kept as raw bytes so invalid UTF-8 can be preserved.

    let result = parser::for_each_token(reader, |token| {
        let token_sym = graph.intern(token)?;

        if let Some(from) = pending.take() {
            graph.add_edge(from, token_sym)?;
        } else {
            pending = Some(token_sym);
        }

        Ok(())
    });

    if let Err(error) = result {
        match error {
            Error::Read(ReadError::Io(e)) if e.kind() == io::ErrorKind::IsADirectory => {
                return Err(ReadError::IsDir(try_clone_str(graph.name())?).into());
            }
            error => return Err(error),
        }
    }

    if pending.is_some() {
        return Err(ReadError::NumTokensOdd(try_clone_str(graph.name())?).into());
    }

    Ok(())
}
