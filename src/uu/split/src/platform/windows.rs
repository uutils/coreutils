// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::ffi::OsStr;
use std::io::{BufWriter, Error, Result};
use std::io::{ErrorKind, Write};
use std::path::Path;
use uucore::display::Quotable;
use uucore::fs;
use uucore::translate;

/// Get a file writer
///
/// Unlike the unix version of this function, this _always_ returns
/// a file writer
pub fn instantiate_current_writer(
    _filter: Option<&str>,
    input: &OsStr,
    filename: &str,
    is_new: bool,
) -> Result<BufWriter<Box<dyn Write>>> {
    let file = if is_new {
        create_or_truncate_output_file(input, filename)?
    } else {
        // re-open file that we previously created to append to it
        let file = std::fs::OpenOptions::new()
            .append(true)
            .open(Path::new(&filename))
            .map_err(|_| {
                Error::other(translate!("split-error-unable-to-reopen-file", "file" => filename))
            })?;

        if input_and_output_refer_to_same_file(input, &file) {
            return Err(Error::other(
                translate!("split-error-would-overwrite-input", "file" => filename.quote()),
            ));
        }

        file
    };
    Ok(BufWriter::new(Box::new(file) as Box<dyn Write>))
}

fn create_or_truncate_output_file(input: &OsStr, filename: &str) -> Result<std::fs::File> {
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(Path::new(filename))
    {
        Ok(file) => Ok(file),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .open(Path::new(filename))
                .map_err(|err| open_file_error(filename, err.kind()))?;

            if input_and_output_refer_to_same_file(input, &file) {
                return Err(Error::other(
                    translate!("split-error-would-overwrite-input", "file" => filename.quote()),
                ));
            }

            file.set_len(0)
                .map_err(|err| open_file_error(filename, err.kind()))?;
            Ok(file)
        }
        Err(e) => Err(open_file_error(filename, e.kind())),
    }
}

fn open_file_error(filename: &str, kind: ErrorKind) -> Error {
    match kind {
        ErrorKind::IsADirectory => {
            Error::other(translate!("split-error-is-a-directory", "dir" => filename))
        }
        _ => Error::other(translate!("split-error-unable-to-open-file", "file" => filename)),
    }
}

fn input_and_output_refer_to_same_file(input: &OsStr, output: &std::fs::File) -> bool {
    let input_info = if input == "-" {
        fs::FileInformation::from_file(&std::io::stdin())
    } else {
        fs::FileInformation::from_path(Path::new(input), true)
    };

    fs::infos_refer_to_same_file(input_info, fs::FileInformation::from_file(output))
}
pub fn paths_refer_to_same_file(p1: &OsStr, p2: &OsStr) -> bool {
    // Windows doesn't support many of the unix ways of paths being equals
    fs::paths_refer_to_same_file(Path::new(p1), Path::new(p2), true)
}
