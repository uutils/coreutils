// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) seekable seek'd tail'ing ringbuffer ringbuf unwatch
// spell-checker:ignore (ToDO) Uncategorized filehandle Signum memrchr
// spell-checker:ignore (libs) kqueue
// spell-checker:ignore (acronyms)
// spell-checker:ignore (env/flags)
// spell-checker:ignore (jargon) tailable untailable stdlib
// spell-checker:ignore (names)
// spell-checker:ignore (shell/tools)
// spell-checker:ignore (misc)

pub mod args;
pub mod chunks;
mod follow;
mod parse;
mod paths;
mod platform;
pub mod text;

pub use args::uu_app;
use args::{FilterMode, Settings, Signum, parse_args};
use chunks::ReverseChunks;
use follow::Observer;
use memchr::{memchr_iter, memrchr_iter};
use paths::{FileExtTail, HeaderPrinter, Input, InputKind, MetadataExtTail};
use same_file::Handle;
use std::cmp::Ordering;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, ErrorKind, Read, Seek, SeekFrom, Write, stdin, stdout};
use std::path::{Path, PathBuf};
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, get_exit_code, set_exit_code};
use uucore::{show, show_error};

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let settings = parse_args(args)?;

    settings.check_warnings();

    match settings.verify() {
        args::VerificationResult::CannotFollowStdinByName => {
            return Err(USimpleError::new(
                1,
                format!("cannot follow {} by name", text::DASH.quote()),
            ));
        }
        // Exit early if we do not output anything. Note, that this may break a pipe
        // when tail is on the receiving side.
        args::VerificationResult::NoOutput => return Ok(()),
        args::VerificationResult::Ok => {}
    }

    uu_tail(&settings)
}

fn uu_tail(settings: &Settings) -> UResult<()> {
    let mut printer = HeaderPrinter::new(settings.verbose, true);
    let mut observer = Observer::from(settings);

    observer.start(settings)?;
    // Do an initial tail print of each path's content.
    // Add `path` and `reader` to `files` map if `--follow` is selected.
    for input in &settings.inputs.clone() {
        match input.kind() {
            InputKind::Stdin => {
                tail_stdin(settings, &mut printer, input, &mut observer)?;
            }
            InputKind::File(path) if cfg!(unix) && path == &PathBuf::from(text::DEV_STDIN) => {
                tail_stdin(settings, &mut printer, input, &mut observer)?;
            }
            InputKind::File(path) => {
                tail_file(settings, &mut printer, input, path, &mut observer, 0)?;
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
        if !settings.has_only_stdin() || settings.pid != 0 {
            follow::follow(observer, settings)?;
        }
    }

    if get_exit_code() > 0 && paths::stdin_is_bad_fd() {
        show_error!("-: {}", text::BAD_FD);
    }

    Ok(())
}

fn tail_file(
    settings: &Settings,
    header_printer: &mut HeaderPrinter,
    input: &Input,
    path: &Path,
    observer: &mut Observer,
    offset: u64,
) -> UResult<()> {
    if !path.exists() {
        set_exit_code(1);
        show_error!(
            "cannot open '{}' for reading: {}",
            input.display_name,
            text::NO_SUCH_FILE
        );
        observer.add_bad_path(path, input.display_name.as_str(), false)?;
    } else if path.is_dir() {
        set_exit_code(1);

        header_printer.print_input(input);
        let err_msg = "Is a directory".to_string();

        show_error!("error reading '{}': {err_msg}", input.display_name);
        if settings.follow.is_some() {
            let msg = if settings.retry {
                ""
            } else {
                "; giving up on this name"
            };
            show_error!(
                "{}: cannot follow end of this type of file{msg}",
                input.display_name,
            );
        }
        if !observer.follow_name_retry() {
            // skip directory if not retry
            return Ok(());
        }
        observer.add_bad_path(path, input.display_name.as_str(), false)?;
    } else if input.is_tailable() {
        let metadata = path.metadata().ok();
        match File::open(path) {
            Ok(mut file) => {
                header_printer.print_input(input);
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
                observer.add_path(
                    path,
                    input.display_name.as_str(),
                    Some(Box::new(reader)),
                    true,
                )?;
            }
            Err(e) if e.kind() == ErrorKind::PermissionDenied => {
                observer.add_bad_path(path, input.display_name.as_str(), false)?;
                show!(e.map_err_context(|| {
                    format!("cannot open '{}' for reading", input.display_name)
                }));
            }
            Err(e) => {
                observer.add_bad_path(path, input.display_name.as_str(), false)?;
                return Err(e.map_err_context(|| {
                    format!("cannot open '{}' for reading", input.display_name)
                }));
            }
        }
    } else {
        observer.add_bad_path(path, input.display_name.as_str(), false)?;
    }

    Ok(())
}

fn tail_stdin(
    settings: &Settings,
    header_printer: &mut HeaderPrinter,
    input: &Input,
    observer: &mut Observer,
) -> UResult<()> {
    // on macOS, resolve() will always return None for stdin,
    // we need to detect if stdin is a directory ourselves.
    // fstat-ing certain descriptors under /dev/fd fails with
    // bad file descriptor or might not catch directory cases
    // e.g. see the differences between running ls -l /dev/stdin /dev/fd/0
    // on macOS and Linux.
    #[cfg(target_os = "macos")]
    {
        if let Ok(mut stdin_handle) = Handle::stdin() {
            if let Ok(meta) = stdin_handle.as_file_mut().metadata() {
                if meta.file_type().is_dir() {
                    set_exit_code(1);
                    show_error!(
                        "cannot open '{}' for reading: {}",
                        input.display_name,
                        text::NO_SUCH_FILE
                    );
                    return Ok(());
                }
            }
        }
    }

    match input.resolve() {
        // fifo
        Some(path) => {
            let mut stdin_offset = 0;
            if cfg!(unix) {
                // Save the current seek position/offset of a stdin redirected file.
                // This is needed to pass "gnu/tests/tail-2/start-middle.sh"
                if let Ok(mut stdin_handle) = Handle::stdin() {
                    if let Ok(offset) = stdin_handle.as_file_mut().stream_position() {
                        stdin_offset = offset;
                    }
                }
            }
            tail_file(
                settings,
                header_printer,
                input,
                &path,
                observer,
                stdin_offset,
            )?;
        }
        // pipe
        None => {
            header_printer.print_input(input);
            if paths::stdin_is_bad_fd() {
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
            } else {
                let mut reader = BufReader::new(stdin());
                unbounded_tail(&mut reader, settings)?;
                observer.add_stdin(input.display_name.as_str(), Some(Box::new(reader)), true)?;
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
fn forwards_thru_file(
    reader: &mut impl Read,
    num_delimiters: u64,
    delimiter: u8,
) -> io::Result<usize> {
    // If num_delimiters == 0, always return 0.
    if num_delimiters == 0 {
        return Ok(0);
    }
    // Use a 32K buffer.
    let mut buf = [0; 32 * 1024];
    let mut total = 0;
    let mut count = 0;
    // Iterate through the input, using `count` to record the number of times `delimiter`
    // is seen. Once we find `num_delimiters` instances, return the offset of the byte
    // immediately following that delimiter.
    loop {
        match reader.read(&mut buf) {
            // Ok(0) => EoF before we found `num_delimiters` instance of `delimiter`.
            // Return the total number of bytes read in that case.
            Ok(0) => return Ok(total),
            Ok(n) => {
                // Use memchr_iter since it greatly improves search performance.
                for offset in memchr_iter(delimiter, &buf[..n]) {
                    count += 1;
                    if count == num_delimiters {
                        // Return offset of the byte after the `delimiter` instance.
                        return Ok(total + offset + 1);
                    }
                }
                total += n;
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
}

/// Iterate over bytes in the file, in reverse, until we find the
/// `num_delimiters` instance of `delimiter`. The `file` is left seek'd to the
/// position just after that delimiter.
fn backwards_thru_file(file: &mut File, num_delimiters: u64, delimiter: u8) {
    // This variable counts the number of delimiters found in the file
    // so far (reading from the end of the file toward the beginning).
    let mut counter = 0;
    let mut first_slice = true;
    for slice in ReverseChunks::new(file) {
        // Iterate over each byte in the slice in reverse order.
        let mut iter = memrchr_iter(delimiter, &slice);

        // Ignore a trailing newline in the last block, if there is one.
        if first_slice {
            if let Some(c) = slice.last() {
                if *c == delimiter {
                    iter.next();
                }
            }
            first_slice = false;
        }

        // For each byte, increment the count of the number of
        // delimiters found. If we have found more than the specified
        // number of delimiters, terminate the search and seek to the
        // appropriate location in the file.
        for i in iter {
            counter += 1;
            if counter >= num_delimiters {
                // We should never over-count - assert that.
                assert_eq!(counter, num_delimiters);
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
    io::copy(file, &mut stdout).unwrap();
}

fn unbounded_tail<T: Read>(reader: &mut BufReader<T>, settings: &Settings) -> UResult<()> {
    let mut writer = BufWriter::new(stdout().lock());
    match &settings.mode {
        FilterMode::Lines(Signum::Negative(count), sep) => {
            let mut chunks = chunks::LinesChunkBuffer::new(*sep, *count);
            chunks.fill(reader)?;
            chunks.print(&mut writer)?;
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
            chunks.print(&mut writer)?;
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
    writer.flush()?;
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
