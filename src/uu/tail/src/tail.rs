//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Morten Olsen Lysgaard <morten@lysgaard.no>
//  * (c) Alexander Batischev <eual.jp@gmail.com>
//  * (c) Thomas Queiroz <thomasqueirozb@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) seekable seek'd tail'ing ringbuffer ringbuf unwatch Uncategorized filehandle Signum
// spell-checker:ignore (libs) kqueue
// spell-checker:ignore (acronyms)
// spell-checker:ignore (env/flags)
// spell-checker:ignore (jargon) tailable untailable stdlib
// spell-checker:ignore (names)
// spell-checker:ignore (shell/tools)
// spell-checker:ignore (misc)

#[macro_use]
extern crate clap;

#[macro_use]
extern crate uucore;
extern crate core;

pub mod args;
pub mod chunks;
mod follow;
mod parse;
mod paths;
mod platform;
pub mod text;

use same_file::Handle;
use std::cmp::Ordering;
use std::fs::File;
use std::io::{self, stdin, stdout, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use uucore::display::Quotable;
use uucore::error::{get_exit_code, set_exit_code, FromIo, UError, UResult, USimpleError};

pub use args::uu_app;
use args::{parse_args, FilterMode, Settings, Signum};
use chunks::ReverseChunks;
use follow::WatcherService;
use paths::{FileExtTail, Input, InputKind, InputService, MetadataExtTail};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let settings = parse_args(args)?;
    uu_tail(&settings)
}

fn uu_tail(settings: &Settings) -> UResult<()> {
    // Mimic GNU's tail for `tail -F` and exit immediately
    let mut input_service = InputService::from(settings);
    let mut watcher_service = WatcherService::from(settings);

    if input_service.has_stdin() && watcher_service.follow_name() {
        return Err(USimpleError::new(
            1,
            format!("cannot follow {} by name", text::DASH.quote()),
        ));
    }

    watcher_service.start(settings)?;
    // Do an initial tail print of each path's content.
    // Add `path` and `reader` to `files` map if `--follow` is selected.
    for input in &input_service.inputs.clone() {
        match input.kind() {
            InputKind::File(path) if cfg!(not(unix)) || path != &PathBuf::from(text::DEV_STDIN) => {
                tail_file(
                    settings,
                    &mut input_service,
                    input,
                    path,
                    &mut watcher_service,
                    0,
                )?;
            }
            // File points to /dev/stdin here
            InputKind::File(_) | InputKind::Stdin => {
                tail_stdin(settings, &mut input_service, input, &mut watcher_service)?;
            }
        }
    }

    if settings.follow.is_some() {
        /*
        POSIX specification regarding tail -f
        If the input file is a regular file or if the file operand specifies a FIFO, do not
        terminate after the last line of the input file has been copied, but read and copy
        further bytes from the input file when they become available. If no file operand is
        specified and standard input is a pipe or FIFO, the -f option shall be ignored. If
        the input file is not a FIFO, pipe, or regular file, it is unspecified whether or
        not the -f option shall be ignored.
        */

        if !input_service.has_only_stdin() {
            follow::follow(watcher_service, settings)?;
        }
    }

    if get_exit_code() > 0 && paths::stdin_is_bad_fd() {
        show_error!("-: {}", text::BAD_FD);
    }

    Ok(())
}

fn tail_file(
    settings: &Settings,
    input_service: &mut InputService,
    input: &Input,
    path: &Path,
    watcher_service: &mut WatcherService,
    offset: u64,
) -> UResult<()> {
    if watcher_service.follow_descriptor_retry() {
        show_warning!("--retry only effective for the initial open");
    }

    if !path.exists() {
        set_exit_code(1);
        show_error!(
            "cannot open '{}' for reading: {}",
            input.display_name,
            text::NO_SUCH_FILE
        );
        watcher_service.add_bad_path(path, input.display_name.as_str(), false)?;
    } else if path.is_dir() {
        set_exit_code(1);

        input_service.print_header(input);
        let err_msg = "Is a directory".to_string();

        show_error!("error reading '{}': {}", input.display_name, err_msg);
        if settings.follow.is_some() {
            let msg = if !settings.retry {
                "; giving up on this name"
            } else {
                ""
            };
            show_error!(
                "{}: cannot follow end of this type of file{}",
                input.display_name,
                msg
            );
        }
        if !(watcher_service.follow_name_retry()) {
            // skip directory if not retry
            return Ok(());
        }
        watcher_service.add_bad_path(path, input.display_name.as_str(), false)?;
    } else if input.is_tailable() {
        let metadata = path.metadata().ok();
        match File::open(path) {
            Ok(mut file) => {
                input_service.print_header(input);
                let mut reader;
                if !settings.presume_input_pipe
                    && file.is_seekable(if input.is_stdin() { offset } else { 0 })
                    && metadata.as_ref().unwrap().get_block_size() > 0
                {
                    bounded_tail(&mut file, settings);
                    reader = BufReader::new(file);
                } else {
                    reader = BufReader::new(file);
                    unbounded_tail(&mut reader, settings)?;
                }
                watcher_service.add_path(
                    path,
                    input.display_name.as_str(),
                    Some(Box::new(reader)),
                    true,
                )?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                watcher_service.add_bad_path(path, input.display_name.as_str(), false)?;
                show!(e.map_err_context(|| {
                    format!("cannot open '{}' for reading", input.display_name)
                }));
            }
            Err(e) => {
                watcher_service.add_bad_path(path, input.display_name.as_str(), false)?;
                return Err(e.map_err_context(|| {
                    format!("cannot open '{}' for reading", input.display_name)
                }));
            }
        }
    } else {
        watcher_service.add_bad_path(path, input.display_name.as_str(), false)?;
    }

    Ok(())
}

fn tail_stdin(
    settings: &Settings,
    input_service: &mut InputService,
    input: &Input,
    watcher_service: &mut WatcherService,
) -> UResult<()> {
    match input.resolve() {
        // fifo
        Some(path) => {
            let mut stdin_offset = 0;
            if cfg!(unix) {
                // Save the current seek position/offset of a stdin redirected file.
                // This is needed to pass "gnu/tests/tail-2/start-middle.sh"
                if let Ok(mut stdin_handle) = Handle::stdin() {
                    if let Ok(offset) = stdin_handle.as_file_mut().seek(SeekFrom::Current(0)) {
                        stdin_offset = offset;
                    }
                }
            }
            tail_file(
                settings,
                input_service,
                input,
                &path,
                watcher_service,
                stdin_offset,
            )?;
        }
        // pipe
        None => {
            input_service.print_header(input);
            if !paths::stdin_is_bad_fd() {
                let mut reader = BufReader::new(stdin());
                unbounded_tail(&mut reader, settings)?;
                watcher_service.add_stdin(
                    input.display_name.as_str(),
                    Some(Box::new(reader)),
                    true,
                )?;
            } else {
                set_exit_code(1);
                show_error!(
                    "cannot fstat {}: {}",
                    text::STDIN_HEADER.quote(),
                    text::BAD_FD
                );
                if settings.follow.is_some() {
                    show_error!(
                        "error reading {}: {}",
                        text::STDIN_HEADER.quote(),
                        text::BAD_FD
                    );
                }
            }
        }
    };

    Ok(())
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
fn forwards_thru_file<R>(
    reader: &mut R,
    num_delimiters: u64,
    delimiter: u8,
) -> std::io::Result<usize>
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
fn backwards_thru_file(file: &mut File, num_delimiters: u64, delimiter: u8) {
    // This variable counts the number of delimiters found in the file
    // so far (reading from the end of the file toward the beginning).
    let mut counter = 0;

    for (block_idx, slice) in ReverseChunks::new(file).enumerate() {
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
                    file.seek(SeekFrom::Current((i + 1) as i64)).unwrap();
                    return;
                }
            }
        }
    }
}

/// When tail'ing a file, we do not need to read the whole file from start to
/// finish just to find the last n lines or bytes. Instead, we can seek to the
/// end of the file, and then read the file "backwards" in blocks of size
/// `BLOCK_SIZE` until we find the location of the first line/byte. This ends up
/// being a nice performance win for very large files.
fn bounded_tail(file: &mut File, settings: &Settings) {
    debug_assert!(!settings.presume_input_pipe);

    // Find the position in the file to start printing from.
    match &settings.mode {
        FilterMode::Lines(Signum::Negative(count), delimiter) => {
            backwards_thru_file(file, *count, *delimiter);
        }
        FilterMode::Lines(Signum::Positive(count), delimiter) if count > &1 => {
            let i = forwards_thru_file(file, *count - 1, *delimiter).unwrap();
            file.seek(SeekFrom::Start(i as u64)).unwrap();
        }
        FilterMode::Lines(Signum::MinusZero, _) => {
            return;
        }
        FilterMode::Bytes(Signum::Negative(count)) => {
            let len = file.seek(SeekFrom::End(0)).unwrap();
            file.seek(SeekFrom::End(-((*count).min(len) as i64)))
                .unwrap();
        }
        FilterMode::Bytes(Signum::Positive(count)) if count > &1 => {
            // GNU `tail` seems to index bytes and lines starting at 1, not
            // at 0. It seems to treat `+0` and `+1` as the same thing.
            file.seek(SeekFrom::Start(*count - 1)).unwrap();
        }
        FilterMode::Bytes(Signum::MinusZero) => {
            return;
        }
        _ => {}
    }

    // Print the target section of the file.
    let stdout = stdout();
    let mut stdout = stdout.lock();
    std::io::copy(file, &mut stdout).unwrap();
}

fn unbounded_tail<T: Read>(reader: &mut BufReader<T>, settings: &Settings) -> UResult<()> {
    let stdout = stdout();
    let mut writer = BufWriter::new(stdout.lock());
    match &settings.mode {
        FilterMode::Lines(Signum::Negative(count), sep) => {
            let mut chunks = chunks::LinesChunkBuffer::new(*sep, *count);
            chunks.fill(reader)?;
            chunks.print(writer)?;
        }
        FilterMode::Lines(Signum::PlusZero | Signum::Positive(1), _) => {
            io::copy(reader, &mut writer)?;
        }
        FilterMode::Lines(Signum::Positive(count), sep) => {
            let mut num_skip = *count - 1;
            let mut chunk = chunks::LinesChunk::new(*sep);
            while chunk.fill(reader)?.is_some() {
                let lines = chunk.get_lines() as u64;
                if lines < num_skip {
                    num_skip -= lines;
                } else {
                    break;
                }
            }
            if chunk.has_data() {
                chunk.print_lines(&mut writer, num_skip as usize)?;
                io::copy(reader, &mut writer)?;
            }
        }
        FilterMode::Bytes(Signum::Negative(count)) => {
            let mut chunks = chunks::BytesChunkBuffer::new(*count);
            chunks.fill(reader)?;
            chunks.print(writer)?;
        }
        FilterMode::Bytes(Signum::PlusZero | Signum::Positive(1)) => {
            io::copy(reader, &mut writer)?;
        }
        FilterMode::Bytes(Signum::Positive(count)) => {
            let mut num_skip = *count - 1;
            let mut chunk = chunks::BytesChunk::new();
            loop {
                if let Some(bytes) = chunk.fill(reader)? {
                    let bytes: u64 = bytes as u64;
                    match bytes.cmp(&num_skip) {
                        Ordering::Less => num_skip -= bytes,
                        Ordering::Equal => {
                            break;
                        }
                        Ordering::Greater => {
                            writer.write_all(chunk.get_buffer_with(num_skip as usize))?;
                            break;
                        }
                    }
                } else {
                    return Ok(());
                }
            }

            io::copy(reader, &mut writer)?;
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use crate::forwards_thru_file;
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
