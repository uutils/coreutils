// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::platform::open_file_error;
use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io::{Error, ErrorKind, Result};
use std::path::Path;
use uucore::display::Quotable;
use uucore::fs::{FileInformation, infos_refer_to_same_file};
use uucore::translate;

/// Open an output file: create or truncate it when `is_new`, otherwise
/// re-open the file we previously created to append to it.
pub fn open_output_file(input: &OsStr, filename: &OsStr, is_new: bool) -> Result<File> {
    if is_new {
        return create_or_truncate_output_file(input, filename);
    }
    let file = OpenOptions::new()
        .append(true)
        .open(Path::new(filename))
        .map_err(|_| {
            Error::other(translate!(
                "split-error-unable-to-reopen-file",
                "file" => filename.quote()
            ))
        })?;
    check_not_input(input, filename, &file)?;
    Ok(file)
}

fn create_or_truncate_output_file(input: &OsStr, filename: &OsStr) -> Result<File> {
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(Path::new(filename))
    {
        Ok(file) => Ok(file),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(Path::new(filename))
                .map_err(|e| open_file_error(filename, e))?;
            check_not_input(input, filename, &file)?;
            match file.set_len(0) {
                // allow writing to symlink to nonseekable
                Err(e) if e.kind() != ErrorKind::InvalidInput => Err(open_file_error(filename, e)),
                _ => Ok(file),
            }
        }
        Err(e) => Err(open_file_error(filename, e)),
    }
}

fn check_not_input(input: &OsStr, filename: &OsStr, output: &File) -> Result<()> {
    let input_info = if input == "-" {
        FileInformation::from_file(&std::io::stdin())
    } else {
        FileInformation::from_path(Path::new(input), true)
    };
    if infos_refer_to_same_file(input_info, FileInformation::from_file(output)) {
        return Err(Error::other(
            translate!("split-error-would-overwrite-input", "file" => filename.quote()),
        ));
    }
    Ok(())
}
