use std::fs::remove_file;
use std::fs::File;
use std::io::{self, BufRead, BufWriter, Write};

/// Splits a file into severals according to the command line patterns.
///
/// # Errors
///
/// Returns a [`io::Error`] if there is some problem reading/writing from/to a file.
pub fn csplit<T>(options: ::CsplitOptions, patterns: Vec<::Pattern>, input: T) -> io::Result<i32>
where
    T: BufRead,
{
    let mut split_writer = SplitWriter::new(&options)?;
    let mut input_iter = input.lines().enumerate().peekable();
    let mut offset_buffer: OffsetBuffer = Default::default();

    // split the file based on patterns
    for pattern in patterns.into_iter() {
        match pattern {
            ::Pattern::UpToLine(n, mut ex) => {
                while let Some(()) = ex.next() {
                    split_writer.new_writer()?;
                    do_to_match(
                        options.quiet,
                        |ln, _| (ln + 1) % n == 0,
                        0,
                        &mut input_iter,
                        &mut offset_buffer,
                        &mut split_writer,
                        true,
                    )?;
                    // break the execute loop on EOF if the ExecutePattern variant is Always
                    if let None = input_iter.peek() {
                        break;
                    }
                }
            }
            ::Pattern::UpToMatch(regex, offset, mut ex) => {
                while let Some(()) = ex.next() {
                    split_writer.new_writer()?;
                    do_to_match(
                        options.quiet,
                        |_, line: &String| regex.is_match(line),
                        offset,
                        &mut input_iter,
                        &mut offset_buffer,
                        &mut split_writer,
                        true,
                    )?;
                    // break the execute loop on EOF if the ExecutePattern variant is Always
                    if input_iter.peek().is_none() {
                        break;
                    }
                }
            }
            ::Pattern::SkipToMatch(regex, offset, mut ex) => {
                while let Some(()) = ex.next() {
                    split_writer.clear_pending_lines();
                    do_to_match(
                        true,
                        |_, line: &String| regex.is_match(line),
                        offset,
                        &mut input_iter,
                        &mut offset_buffer,
                        &mut split_writer,
                        false,
                    )?;
                    // break the execute loop on EOF if the ExecutePattern variant is Always
                    if input_iter.peek().is_none() {
                        break;
                    }
                }
            }
        };
    }
    // consume the rest
    if let Some((_, line)) = input_iter.next() {
        split_writer.new_writer()?;
        split_writer.writeln(line?)?;
        for (_, line) in input_iter {
            split_writer.writeln(line?)?;
        }
        if !options.quiet {
            println!("{}", split_writer.size);
        }
    }
    Ok(0)
}

/// Read lines up to the matching line and act on them. With a non-zero offset, the block of
/// relevant lines can be extended (if positive), or reduced (if negative).
///
/// If `write_line` is true, the relevant lines are written to a new file.
///
/// # Errors
///
/// If there were errors reading/writing from/to a file.
fn do_to_match<I, F>(
    quiet: bool,
    is_match: F,
    offset: i32,
    input_iter: &mut I,
    offset_buffer: &mut OffsetBuffer,
    split_writer: &mut SplitWriter,
    write_line: bool,
) -> io::Result<()>
where
    I: Iterator<Item = (usize, io::Result<String>)>,
    F: Fn(usize, &String) -> bool,
{
    if offset >= 0 {
        // the offset is zero or positive, no need for a buffer on the lines read
        let mut offset_copy = offset;
        while let Some((ln, line)) = input_iter.next() {
            let l = line?;
            if is_match(ln, &l) {
                if offset_copy == 0 {
                    split_writer.add_pending_line(l);
                } else {
                    offset_copy -= 1;
                    if write_line {
                        split_writer.writeln(l)?;
                    }
                }
                break;
            }
            if write_line {
                split_writer.writeln(l)?;
            }
        }
        // write the extra lines required by the offset
        while offset_copy > 0 {
            offset_copy -= 1;
            match input_iter.next() {
                Some((_, line)) => {
                    if write_line {
                        split_writer.writeln(line?)?;
                    }
                }
                None => break,
            };
        }
    } else {
        // with a negative offset we use a buffer to keep the lines within the offset
        offset_buffer.set_size(-offset as usize);
        while let Some((ln, line)) = input_iter.next() {
            let l = line?;
            if is_match(ln, &l) {
                for line in offset_buffer.drain() {
                    split_writer.add_pending_line(line);
                }
                split_writer.add_pending_line(l);
                break;
            }
            if let Some(line) = offset_buffer.add_line(l) {
                if write_line {
                    split_writer.writeln(line)?;
                }
            }
        }
    }

    if !quiet {
        println!("{}", split_writer.size);
    }
    Ok(())
}

/// The OffsetBuffer retains an `offset` number of lines read, in order to conditionally write them
/// to a split.
///
/// It is used when the offset is negative, in order to write the lines within the offset just before
/// the match into the next split. Lines that spill over the buffer are either written to the current
/// split with [`Pattern::UpToMatch`] or ignored with [`Pattern::SkipToMatch`].
#[derive(Default)]
struct OffsetBuffer {
    buffer: Vec<String>,
    size: usize,
}

impl OffsetBuffer {
    /// Set the maximum number of lines to keep.
    fn set_size(&mut self, size: usize) {
        self.size = size;
    }

    /// Add a line to the buffer. If the buffer has [`size`] elements, then its head is removed and
    /// the new line is pushed to the buffer. The removed head is then available in the returned
    /// option.
    fn add_line(&mut self, line: String) -> Option<String> {
        if self.buffer.len() == self.size {
            let head = self.buffer.remove(0);
            self.buffer.push(line);
            Some(head)
        } else {
            self.buffer.push(line);
            None
        }
    }

    /// Returns an iterator that drains the content of the buffer.
    fn drain<'a>(&'a mut self) -> impl Iterator<Item = String> + 'a {
        self.buffer.drain(..)
    }
}

/// Write a portion of the input file into a split which filename is based on an incrementing
/// counter.
struct SplitWriter<'a> {
    /// the options set through the command line
    options: &'a ::CsplitOptions,
    /// a split counter
    counter: usize,
    /// the writer to the current split
    current_writer: BufWriter<File>,
    /// list of lines to be written to a newly created split
    pending_lines: Vec<String>,
    /// the size in bytes of the current split
    size: usize,
}

impl<'a> Drop for SplitWriter<'a> {
    fn drop(&mut self) {
        // if still in the initialization stage, then there was nothing to write to a split
        // and so we clean the file created in new().
        if self.counter == 0 {
            let file_name = self.options.prefix.to_owned() + "0";
            remove_file(file_name).unwrap();
        }
    }
}

impl<'a> SplitWriter<'a> {
    fn new(options: &::CsplitOptions) -> io::Result<SplitWriter> {
        let file_name = options.prefix.to_owned() + "0";
        let file = File::create(file_name)?;
        Ok(SplitWriter {
            options,
            counter: 0,
            current_writer: BufWriter::new(file),
            pending_lines: Vec::new(),
            size: 0,
        })
    }

    /// Creates a new split.
    ///
    /// # Errors
    ///
    /// The creation of the split file, or the draining of the pending lines to it, may fail with
    /// some [`io::Error`].
    fn new_writer(&mut self) -> io::Result<()> {
        if self.counter != 0 {
            let file_name = self.options.prefix.to_owned() + &self.counter.to_string();
            let file = File::create(file_name)?;
            self.current_writer = BufWriter::new(file);
        }
        self.counter += 1;
        self.size = 0;
        for line in self.pending_lines.drain(..) {
            let bytes = line.as_bytes();
            self.current_writer.write_all(bytes)?;
            self.current_writer.write(b"\n")?;
            self.size += bytes.len() + 1;
        }
        Ok(())
    }

    /// Writes the line to the current split, appending a newline character.
    ///
    /// It returns the number of bytes written if successful.
    ///
    /// # Errors
    ///
    /// Some [`io::Error`] may occur when attempting to write the line.
    fn writeln(&mut self, line: String) -> io::Result<()> {
        let bytes = line.as_bytes();
        self.current_writer.write_all(bytes)?;
        self.current_writer.write(b"\n")?;
        self.size += bytes.len() + 1;
        Ok(())
    }

    /// Remove any pending lines that would be written to the next split.
    fn clear_pending_lines(&mut self) {
        self.pending_lines.clear();
    }

    /// Add a line to be written to the next split.
    fn add_pending_line(&mut self, line: String) {
        self.pending_lines.push(line);
    }
}
