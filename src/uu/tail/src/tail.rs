// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) seekable seek'd tail'ing ringbuffer ringbuf unwatch Uncategorized filehandle Signum
// spell-checker:ignore (libs) kqueue
// spell-checker:ignore (acronyms)
// spell-checker:ignore (env/flags)
// spell-checker:ignore (jargon) tailable untailable stdlib
// spell-checker:ignore (names)
// spell-checker:ignore (shell/tools)
// spell-checker:ignore (misc)

pub mod args;
pub mod chunks;
mod error;
mod follow;
mod parse;
mod paths;
mod platform;
pub mod text;

pub use args::uu_app;
use args::{parse_args, FilterMode, Settings, Signum};
use chunks::ReverseChunks;
use error::{new_io_directory_error, TailError, TailErrorHandler};
use follow::Observer;
use paths::{FileExtTail, HeaderPrinter, Input, MetadataExtTail, Opened};
use std::cmp::Ordering;
use std::fs::File;
use std::io::{self, stdin, stdout, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let settings = parse_args(args)?;

    settings.check_warnings();

    match settings.verify() {
        args::VerificationResult::CannotFollowStdinByName => {
            return Err(USimpleError::new(
                1,
                format!("cannot follow {} by name", text::DASH.quote()),
            ))
        }
        // Exit early if we do not output anything. Note, that this may break a pipe
        // when tail is on the receiving side.
        args::VerificationResult::NoOutput => return Ok(()),
        args::VerificationResult::Ok => {}
    }

    uu_tail(&settings)
}

fn uu_tail(settings: &Settings) -> UResult<()> {
    let mut header_printer = HeaderPrinter::new(settings.verbose, true);
    let mut observer = Observer::from(settings);

    observer.start(settings)?;

    for input in &settings.inputs {
        tail_input(input, settings, &mut header_printer, &mut observer)?;
    }

    follow::follow(observer, settings)
}

// Do an initial tail print of each path's content.
// Add `path` and `reader` to `files` map if `--follow` is selected.
fn tail_input(
    input: &Input,
    settings: &Settings,
    header_printer: &mut HeaderPrinter,
    observer: &mut Observer,
) -> UResult<()> {
    let result: Result<File, (Option<File>, TailError)> = match input.open() {
        Ok(Opened::File(mut file)) => match tail_file(settings, header_printer, input, &mut file) {
            Ok(_) => Ok(file),
            Err(error) => Err((Some(file), error)),
        },
        // looks like windows doesn't handle file seeks properly when the file is a fifo so we
        // use unbounded tail (in tail_stdin()) which does the right thing.
        Ok(Opened::Fifo(mut file)) if !cfg!(windows) => {
            match tail_file(settings, header_printer, input, &mut file) {
                Ok(_) => Ok(file),
                Err(error) => Err((Some(file), error)),
            }
        }
        Ok(Opened::Fifo(file)) | Ok(Opened::Pipe(file)) => {
            match tail_stdin(settings, header_printer, input) {
                Ok(_) => Ok(file),
                Err(error) => Err((Some(file), error)),
            }
        }
        // Unlike unix systems, windows returns an error (Permission denied) when trying to open
        // directories like regular files (i.e. with File::open). So we have to fix that to
        // maintain compatible behavior with the unix version of uu_tail
        Err(error) if cfg!(windows) => {
            if let Some(meta) = input.path().and_then(|path| path.metadata().ok()) {
                if meta.is_dir() {
                    header_printer.print_input(input);
                    Err((None, TailError::Read(new_io_directory_error())))
                } else {
                    Err((None, TailError::Open(error)))
                }
            } else {
                Err((None, TailError::Open(error)))
            }
        }
        // We do not print the header if we were unable to open the file to match gnu's tail
        // behavior
        Err(error) => Err((None, TailError::Open(error))),
    };

    match result {
        Ok(file) if input.is_stdin() => {
            let reader = Box::new(BufReader::new(file));
            observer.add_stdin(input.display_name.as_str(), Some(reader), true)?;
        }
        Ok(file) => {
            let reader = Box::new(BufReader::new(file));
            observer.add_path(
                // we can safely unwrap here because path is None only on windows if input is
                // stdin
                input.path().unwrap().as_path(),
                input.display_name.as_str(),
                Some(reader),
                true,
            )?;
        }
        Err((file, error)) => {
            let handler = TailErrorHandler::from(input.clone(), observer);
            handler.handle(&error, file, observer)?;
        }
    }
    Ok(())
}

fn tail_file(
    settings: &Settings,
    header_printer: &mut HeaderPrinter,
    input: &Input,
    file: &mut File,
) -> Result<u64, TailError> {
    header_printer.print_input(input);

    let meta = file.metadata().map_err(TailError::Stat)?;

    if !settings.presume_input_pipe && file.is_seekable() && meta.get_block_size() > 0 {
        bounded_tail(file, settings)
    } else {
        let mut reader = BufReader::new(file);
        unbounded_tail(&mut reader, settings)
    }
}

fn tail_stdin(
    settings: &Settings,
    header_printer: &mut HeaderPrinter,
    input: &Input,
) -> Result<u64, TailError> {
    header_printer.print_input(input);
    unbounded_tail(&mut BufReader::new(stdin()), settings)
}

/// Find the index after the given number of instances of a given byte.
///
/// This function reads through a given reader until `num_delimiters`
/// instances of `delimiter` have been seen, returning the index of
/// the byte immediately following that delimiter. If there are fewer
/// than `num_delimiters` instances of `delimiter`, this returns the
/// total number of bytes read from the `reader` until EOF.
///
/// # Errors
///
/// This function returns an error if there is an error during reading
/// from `reader`.
///
/// # Examples
///
/// Basic usage:
///
/// ```rust,ignore
/// use std::io::Cursor;
///
/// let mut reader = Cursor::new("a\nb\nc\nd\ne\n");
/// let i = forwards_thru_file(&mut reader, 2, b'\n').unwrap();
/// assert_eq!(i, 4);
/// ```
///
/// If `num_delimiters` is zero, then this function always returns
/// zero:
///
/// ```rust,ignore
/// use std::io::Cursor;
///
/// let mut reader = Cursor::new("a\n");
/// let i = forwards_thru_file(&mut reader, 0, b'\n').unwrap();
/// assert_eq!(i, 0);
/// ```
///
/// If there are fewer than `num_delimiters` instances of `delimiter`
/// in the reader, then this function returns the total number of
/// bytes read:
///
/// ```rust,ignore
/// use std::io::Cursor;
///
/// let mut reader = Cursor::new("a\n");
/// let i = forwards_thru_file(&mut reader, 2, b'\n').unwrap();
/// assert_eq!(i, 2);
/// ```
fn forwards_thru_file<R>(reader: &mut R, num_delimiters: u64, delimiter: u8) -> io::Result<usize>
where
    R: Read,
{
    let mut reader = BufReader::new(reader);

    let mut buf = vec![];
    let mut total = 0;
    for _ in 0..num_delimiters {
        match reader.read_until(delimiter, &mut buf) {
            Ok(0) => {
                return Ok(total);
            }
            Ok(n) => {
                total += n;
                buf.clear();
                continue;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    Ok(total)
}

/// Iterate over bytes in the file, in reverse, until we find the
/// `num_delimiters` instance of `delimiter`. The `file` is left seek'd to the
/// position just after that delimiter.
fn backwards_thru_file(file: &mut File, num_delimiters: u64, delimiter: u8) -> io::Result<()> {
    // This variable counts the number of delimiters found in the file
    // so far (reading from the end of the file toward the beginning).
    let mut counter = 0;

    for (block_idx, slice) in ReverseChunks::new(file)?.enumerate() {
        let slice = slice?;

        // Iterate over each byte in the slice in reverse order.
        let mut iter = slice.iter().enumerate().rev();

        // Ignore a trailing newline in the last block, if there is one.
        if block_idx == 0 {
            if let Some(c) = slice.last() {
                if *c == delimiter {
                    iter.next();
                }
            }
        }

        // For each byte, increment the count of the number of
        // delimiters found. If we have found more than the specified
        // number of delimiters, terminate the search and seek to the
        // appropriate location in the file.
        for (i, ch) in iter {
            if *ch == delimiter {
                counter += 1;
                if counter >= num_delimiters {
                    // After each iteration of the outer loop, the
                    // cursor in the file is at the *beginning* of the
                    // block, so seeking forward by `i + 1` bytes puts
                    // us right after the found delimiter.
                    file.seek(SeekFrom::Current((i + 1) as i64))?;
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

/// A helper function to copy the whole content of the `reader` into the `writer`.
///
/// To differentiate between read and write errors this function initially tries to read zero bytes
/// from the reader. This distinction is necessary, since write errors lead to the abortion of the
/// whole program and read errors do not. If the initial read fails a [`TailError::Read`] is
/// returned. If [`std::io::copy`] fails it'll return a [`TailError::Write`].
fn copy(reader: &mut impl Read, writer: &mut impl Write) -> Result<u64, TailError> {
    let mut zero_buffer = vec![];
    reader
        .read(zero_buffer.as_mut_slice())
        .map_err(TailError::Read)?;
    io::copy(reader, writer).map_err(TailError::Write)
}

/// When tail'ing a file, we do not need to read the whole file from start to
/// finish just to find the last n lines or bytes. Instead, we can seek to the
/// end of the file, and then read the file "backwards" in blocks of size
/// `BLOCK_SIZE` until we find the location of the first line/byte. This ends up
/// being a nice performance win for very large files.
fn bounded_tail(file: &mut File, settings: &Settings) -> Result<u64, TailError> {
    debug_assert!(!settings.presume_input_pipe);

    // Find the position in the file to start printing from.
    match &settings.mode {
        FilterMode::Lines(Signum::Negative(count), delimiter) => {
            backwards_thru_file(file, *count, *delimiter).map_err(TailError::Read)?;
        }
        FilterMode::Lines(Signum::Positive(count), delimiter) if count > &1 => {
            let i = forwards_thru_file(file, *count - 1, *delimiter).map_err(TailError::Read)?;
            file.seek(SeekFrom::Start(i as u64))
                .map_err(TailError::Read)?;
        }
        FilterMode::Lines(Signum::MinusZero, _) => {
            return Ok(0);
        }
        FilterMode::Bytes(Signum::Negative(count)) => {
            let len = file.seek(SeekFrom::End(0)).map_err(TailError::Read)?;
            file.seek(SeekFrom::End(-((*count).min(len) as i64)))
                .map_err(TailError::Read)?;
        }
        FilterMode::Bytes(Signum::Positive(count)) if count > &1 => {
            // GNU `tail` seems to index bytes and lines starting at 1, not
            // at 0. It seems to treat `+0` and `+1` as the same thing.
            file.seek(SeekFrom::Start(*count - 1))
                .map_err(TailError::Read)?;
        }
        FilterMode::Bytes(Signum::MinusZero) => {
            return Ok(0);
        }
        _ => {}
    }

    // Print the target section of the file.
    let mut stdout = stdout().lock();
    let bytes = io::copy(file, &mut stdout).map_err(TailError::Write)?;
    stdout.flush().map_err(TailError::Write)?;
    Ok(bytes)
}

fn unbounded_tail<T: Read>(
    reader: &mut BufReader<T>,
    settings: &Settings,
) -> Result<u64, TailError> {
    let stdout = stdout();
    let mut writer = BufWriter::new(stdout.lock());
    let result = match &settings.mode {
        FilterMode::Lines(Signum::Negative(count), sep) => {
            let mut chunks = chunks::LinesChunkBuffer::new(*sep, *count);
            let bytes = chunks.fill(reader).map_err(TailError::Read)?;
            chunks.print(&mut writer).map_err(TailError::Write)?;
            Ok(bytes)
        }
        FilterMode::Lines(Signum::PlusZero | Signum::Positive(1), _) => copy(reader, &mut writer),
        FilterMode::Lines(Signum::Positive(count), sep) => {
            let mut num_skip = *count - 1;
            let mut chunk = chunks::LinesChunk::new(*sep);
            while chunk.fill(reader).map_err(TailError::Read)?.is_some() {
                let lines = chunk.get_lines() as u64;
                if lines < num_skip {
                    num_skip -= lines;
                } else {
                    break;
                }
            }
            if chunk.has_data() {
                let bytes = chunk
                    .print_lines(&mut writer, num_skip as usize)
                    .map_err(TailError::Write)?;
                copy(reader, &mut writer).map(|b| b + bytes as u64)
            } else {
                Ok(0)
            }
        }
        FilterMode::Bytes(Signum::Negative(count)) => {
            let mut chunks = chunks::BytesChunkBuffer::new(*count);
            let bytes = chunks.fill(reader).map_err(TailError::Read)?;
            chunks.print(&mut writer).map_err(TailError::Write)?;
            Ok(bytes)
        }
        FilterMode::Bytes(Signum::PlusZero | Signum::Positive(1)) => copy(reader, &mut writer),
        FilterMode::Bytes(Signum::Positive(count)) => {
            let mut num_skip = *count - 1;
            let mut chunk = chunks::BytesChunk::new();
            let mut sum_bytes = 0u64;
            let bytes = loop {
                if let Some(bytes) = chunk.fill(reader).map_err(TailError::Read)? {
                    let bytes: u64 = bytes as u64;
                    match bytes.cmp(&num_skip) {
                        Ordering::Less => num_skip -= bytes,
                        Ordering::Equal => {
                            break None;
                        }
                        Ordering::Greater => {
                            let buffer = chunk.get_buffer_with(num_skip as usize);
                            writer.write_all(buffer).map_err(TailError::Write)?;
                            sum_bytes += buffer.len() as u64;
                            break None;
                        }
                    }
                } else {
                    break Some(sum_bytes);
                }
            };

            if let Some(bytes) = bytes {
                Ok(bytes)
            } else {
                copy(reader, &mut writer).map(|b| b + sum_bytes)
            }
        }
        _ => Ok(0),
    };

    writer.flush().map_err(TailError::Write)?;
    result
}

#[cfg(test)]
mod tests {

    use crate::*;
    use std::io::Cursor;

    #[test]
    fn test_forwards_thru_file_zero() {
        let mut reader = Cursor::new("a\n");
        let i = forwards_thru_file(&mut reader, 0, b'\n').unwrap();
        assert_eq!(i, 0);
    }

    #[test]
    fn test_forwards_thru_file_basic() {
        //                   01 23 45 67 89
        let mut reader = Cursor::new("a\nb\nc\nd\ne\n");
        let i = forwards_thru_file(&mut reader, 2, b'\n').unwrap();
        assert_eq!(i, 4);
    }

    #[test]
    fn test_forwards_thru_file_past_end() {
        let mut reader = Cursor::new("x\n");
        let i = forwards_thru_file(&mut reader, 2, b'\n').unwrap();
        assert_eq!(i, 2);
    }
}
