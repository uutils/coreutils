#![crate_name = "uu_csplit"]

#[macro_use]
extern crate uucore;
use getopts::Matches;
use regex::Regex;
use std::cmp::Ordering;
use std::io::{self, BufReader};
use std::{
    fs::{remove_file, File},
    io::{BufRead, BufWriter, Write},
};

mod csplit_error;
mod patterns;
mod splitname;

use crate::csplit_error::CsplitError;
use crate::splitname::SplitName;

static SYNTAX: &str = "[OPTION]... FILE PATTERN...";
static SUMMARY: &str = "split a file into sections determined by context lines";
static LONG_HELP: &str = "Output pieces of FILE separated by PATTERN(s) to files 'xx00', 'xx01', ..., and output byte counts of each piece to standard output.";

static SUFFIX_FORMAT_OPT: &str = "suffix-format";
static SUPPRESS_MATCHED_OPT: &str = "suppress-matched";
static DIGITS_OPT: &str = "digits";
static PREFIX_OPT: &str = "prefix";
static KEEP_FILES_OPT: &str = "keep-files";
static QUIET_OPT: &str = "quiet";
static ELIDE_EMPTY_FILES_OPT: &str = "elide-empty-files";

/// Command line options for csplit.
pub struct CsplitOptions {
    split_name: crate::SplitName,
    keep_files: bool,
    quiet: bool,
    elide_empty_files: bool,
    suppress_matched: bool,
}

impl CsplitOptions {
    fn new(matches: &Matches) -> CsplitOptions {
        let keep_files = matches.opt_present(KEEP_FILES_OPT);
        let quiet = matches.opt_present(QUIET_OPT);
        let elide_empty_files = matches.opt_present(ELIDE_EMPTY_FILES_OPT);
        let suppress_matched = matches.opt_present(SUPPRESS_MATCHED_OPT);

        CsplitOptions {
            split_name: crash_if_err!(
                1,
                SplitName::new(
                    matches.opt_str(PREFIX_OPT),
                    matches.opt_str(SUFFIX_FORMAT_OPT),
                    matches.opt_str(DIGITS_OPT)
                )
            ),
            keep_files,
            quiet,
            elide_empty_files,
            suppress_matched,
        }
    }
}

/// Splits a file into severals according to the command line patterns.
///
/// # Errors
///
/// - [`io::Error`] if there is some problem reading/writing from/to a file.
/// - [`::CsplitError::LineOutOfRange`] if the linenum pattern is larger than the number of input
///   lines.
/// - [`::CsplitError::LineOutOfRangeOnRepetition`], like previous but after applying the pattern
///   more than once.
/// - [`::CsplitError::MatchNotFound`] if no line matched a regular expression.
/// - [`::CsplitError::MatchNotFoundOnRepetition`], like previous but after applying the pattern
///   more than once.
pub fn csplit<T>(
    options: &CsplitOptions,
    patterns: Vec<patterns::Pattern>,
    input: T,
) -> Result<(), CsplitError>
where
    T: BufRead,
{
    let mut input_iter = InputSplitter::new(input.lines().enumerate());
    let mut split_writer = SplitWriter::new(&options);
    let ret = do_csplit(&mut split_writer, patterns, &mut input_iter);

    // consume the rest
    input_iter.rewind_buffer();
    if let Some((_, line)) = input_iter.next() {
        split_writer.new_writer()?;
        split_writer.writeln(line?)?;
        for (_, line) in input_iter {
            split_writer.writeln(line?)?;
        }
        split_writer.finish_split();
    }
    // delete files on error by default
    if ret.is_err() && !options.keep_files {
        split_writer.delete_all_splits()?;
    }
    ret
}

fn do_csplit<I>(
    split_writer: &mut SplitWriter,
    patterns: Vec<patterns::Pattern>,
    input_iter: &mut InputSplitter<I>,
) -> Result<(), CsplitError>
where
    I: Iterator<Item = (usize, io::Result<String>)>,
{
    // split the file based on patterns
    for pattern in patterns.into_iter() {
        let pattern_as_str = pattern.to_string();
        #[allow(clippy::match_like_matches_macro)]
        let is_skip = if let patterns::Pattern::SkipToMatch(_, _, _) = pattern {
            true
        } else {
            false
        };
        match pattern {
            patterns::Pattern::UpToLine(n, ex) => {
                let mut up_to_line = n;
                for (_, ith) in ex.iter() {
                    split_writer.new_writer()?;
                    match split_writer.do_to_line(&pattern_as_str, up_to_line, input_iter) {
                        // the error happened when applying the pattern more than once
                        Err(CsplitError::LineOutOfRange(_)) if ith != 1 => {
                            return Err(CsplitError::LineOutOfRangeOnRepetition(
                                pattern_as_str.to_string(),
                                ith - 1,
                            ));
                        }
                        Err(err) => return Err(err),
                        // continue the splitting process
                        Ok(()) => (),
                    }
                    up_to_line += n;
                }
            }
            patterns::Pattern::UpToMatch(regex, offset, ex)
            | patterns::Pattern::SkipToMatch(regex, offset, ex) => {
                for (max, ith) in ex.iter() {
                    if is_skip {
                        // when skipping a part of the input, no writer is created
                        split_writer.as_dev_null();
                    } else {
                        split_writer.new_writer()?;
                    }
                    match (
                        split_writer.do_to_match(&pattern_as_str, &regex, offset, input_iter),
                        max,
                    ) {
                        // in case of ::pattern::ExecutePattern::Always, then it's fine not to find a
                        // matching line
                        (Err(CsplitError::MatchNotFound(_)), None) => {
                            return Ok(());
                        }
                        // the error happened when applying the pattern more than once
                        (Err(CsplitError::MatchNotFound(_)), Some(m)) if m != 1 && ith != 1 => {
                            return Err(CsplitError::MatchNotFoundOnRepetition(
                                pattern_as_str.to_string(),
                                ith - 1,
                            ));
                        }
                        (Err(err), _) => return Err(err),
                        // continue the splitting process
                        (Ok(()), _) => (),
                    };
                }
            }
        };
    }
    Ok(())
}

/// Write a portion of the input file into a split which filename is based on an incrementing
/// counter.
struct SplitWriter<'a> {
    /// the options set through the command line
    options: &'a CsplitOptions,
    /// a split counter
    counter: usize,
    /// the writer to the current split
    current_writer: Option<BufWriter<File>>,
    /// the size in bytes of the current split
    size: usize,
    /// flag to indicate that no content should be written to a split
    dev_null: bool,
}

impl<'a> Drop for SplitWriter<'a> {
    fn drop(&mut self) {
        if self.options.elide_empty_files && self.size == 0 {
            let file_name = self.options.split_name.get(self.counter);
            remove_file(file_name).expect("Failed to elide split");
        }
    }
}

impl<'a> SplitWriter<'a> {
    fn new(options: &CsplitOptions) -> SplitWriter {
        SplitWriter {
            options,
            counter: 0,
            current_writer: None,
            size: 0,
            dev_null: false,
        }
    }

    /// Creates a new split and returns its filename.
    ///
    /// # Errors
    ///
    /// The creation of the split file may fail with some [`io::Error`].
    fn new_writer(&mut self) -> io::Result<()> {
        let file_name = self.options.split_name.get(self.counter);
        let file = File::create(&file_name)?;
        self.current_writer = Some(BufWriter::new(file));
        self.counter += 1;
        self.size = 0;
        self.dev_null = false;
        Ok(())
    }

    /// The current split will not keep any of the read input lines.
    fn as_dev_null(&mut self) {
        self.dev_null = true;
    }

    /// Writes the line to the current split, appending a newline character.
    /// If [`dev_null`] is true, then the line is discarded.
    ///
    /// # Errors
    ///
    /// Some [`io::Error`] may occur when attempting to write the line.
    fn writeln(&mut self, line: String) -> io::Result<()> {
        if !self.dev_null {
            match self.current_writer {
                Some(ref mut current_writer) => {
                    let bytes = line.as_bytes();
                    current_writer.write_all(bytes)?;
                    current_writer.write_all(b"\n")?;
                    self.size += bytes.len() + 1;
                }
                None => panic!("trying to write to a split that was not created"),
            }
        }
        Ok(())
    }

    /// Perform some operations after completing a split, i.e., either remove it
    /// if the [`::ELIDE_EMPTY_FILES_OPT`] option is enabled, or print how much bytes were written
    /// to it if [`::QUIET_OPT`] is disabled.
    ///
    /// # Errors
    ///
    /// Some [`io::Error`] if the split could not be removed in case it should be elided.
    fn finish_split(&mut self) {
        if !self.dev_null {
            if self.options.elide_empty_files && self.size == 0 {
                self.counter -= 1;
            } else if !self.options.quiet {
                println!("{}", self.size);
            }
        }
    }

    /// Removes all the split files that were created.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] if there was a problem removing a split.
    fn delete_all_splits(&self) -> io::Result<()> {
        let mut ret = Ok(());
        for ith in 0..self.counter {
            let file_name = self.options.split_name.get(ith);
            if let Err(err) = remove_file(file_name) {
                ret = Err(err);
            }
        }
        ret
    }

    /// Split the input stream up to the line number `n`.
    ///
    /// If the line number `n` is smaller than the current position in the input, then an empty
    /// split is created.
    ///
    /// # Errors
    ///
    /// In addition to errors reading/writing from/to a file, if the line number
    /// `n` is greater than the total available lines, then a
    /// [`::CsplitError::LineOutOfRange`] error is returned.
    fn do_to_line<I>(
        &mut self,
        pattern_as_str: &str,
        n: usize,
        input_iter: &mut InputSplitter<I>,
    ) -> Result<(), CsplitError>
    where
        I: Iterator<Item = (usize, io::Result<String>)>,
    {
        input_iter.rewind_buffer();
        input_iter.set_size_of_buffer(1);

        let mut ret = Err(CsplitError::LineOutOfRange(pattern_as_str.to_string()));
        while let Some((ln, line)) = input_iter.next() {
            let l = line?;
            match n.cmp(&(&ln + 1)) {
                Ordering::Less => {
                    if input_iter.add_line_to_buffer(ln, l).is_some() {
                        panic!("the buffer is big enough to contain 1 line");
                    }
                    ret = Ok(());
                    break;
                }
                Ordering::Equal => {
                    if !self.options.suppress_matched
                        && input_iter.add_line_to_buffer(ln, l).is_some()
                    {
                        panic!("the buffer is big enough to contain 1 line");
                    }
                    ret = Ok(());
                    break;
                }
                Ordering::Greater => (),
            }
            self.writeln(l)?;
        }
        self.finish_split();
        ret
    }

    /// Read lines up to the line matching a [`Regex`]. With a non-zero offset,
    /// the block of relevant lines can be extended (if positive), or reduced
    /// (if negative).
    ///
    /// # Errors
    ///
    /// In addition to errors reading/writing from/to a file, the following errors may be returned:
    /// - if no line matched, an [`::CsplitError::MatchNotFound`].
    /// - if there are not enough lines to accommodate the offset, an
    /// [`::CsplitError::LineOutOfRange`].
    fn do_to_match<I>(
        &mut self,
        pattern_as_str: &str,
        regex: &Regex,
        mut offset: i32,
        input_iter: &mut InputSplitter<I>,
    ) -> Result<(), CsplitError>
    where
        I: Iterator<Item = (usize, io::Result<String>)>,
    {
        if offset >= 0 {
            // The offset is zero or positive, no need for a buffer on the lines read.
            // NOTE: drain the buffer of input_iter, no match should be done within.
            for line in input_iter.drain_buffer() {
                self.writeln(line)?;
            }
            // retain the matching line
            input_iter.set_size_of_buffer(1);

            while let Some((ln, line)) = input_iter.next() {
                let l = line?;
                if regex.is_match(&l) {
                    match (self.options.suppress_matched, offset) {
                        // no offset, add the line to the next split
                        (false, 0) => {
                            if input_iter.add_line_to_buffer(ln, l).is_some() {
                                panic!("the buffer is big enough to contain 1 line");
                            }
                        }
                        // a positive offset, some more lines need to be added to the current split
                        (false, _) => self.writeln(l)?,
                        _ => (),
                    };
                    offset -= 1;

                    // write the extra lines required by the offset
                    while offset > 0 {
                        match input_iter.next() {
                            Some((_, line)) => {
                                self.writeln(line?)?;
                            }
                            None => {
                                self.finish_split();
                                return Err(CsplitError::LineOutOfRange(
                                    pattern_as_str.to_string(),
                                ));
                            }
                        };
                        offset -= 1;
                    }
                    self.finish_split();
                    return Ok(());
                }
                self.writeln(l)?;
            }
        } else {
            // With a negative offset we use a buffer to keep the lines within the offset.
            // NOTE: do not drain the buffer of input_iter, in case of an LineOutOfRange error
            // but do not rewind it either since no match should be done within.
            // The consequence is that the buffer may already be full with lines from a previous
            // split, which is taken care of when calling `shrink_buffer_to_size`.
            let offset_usize = -offset as usize;
            input_iter.set_size_of_buffer(offset_usize);
            while let Some((ln, line)) = input_iter.next() {
                let l = line?;
                if regex.is_match(&l) {
                    for line in input_iter.shrink_buffer_to_size() {
                        self.writeln(line)?;
                    }
                    if !self.options.suppress_matched {
                        // add 1 to the buffer size to make place for the matched line
                        input_iter.set_size_of_buffer(offset_usize + 1);
                        if input_iter.add_line_to_buffer(ln, l).is_some() {
                            panic!("should be big enough to hold every lines");
                        }
                    }
                    self.finish_split();
                    if input_iter.buffer_len() < offset_usize {
                        return Err(CsplitError::LineOutOfRange(pattern_as_str.to_string()));
                    }
                    return Ok(());
                }
                if let Some(line) = input_iter.add_line_to_buffer(ln, l) {
                    self.writeln(line)?;
                }
            }
            // no match, drain the buffer into the current split
            for line in input_iter.drain_buffer() {
                self.writeln(line)?;
            }
        }

        self.finish_split();
        Err(CsplitError::MatchNotFound(pattern_as_str.to_string()))
    }
}

/// An iterator which can output items from a buffer filled externally.
/// This is used to pass matching lines to the next split and to support patterns with a negative offset.
struct InputSplitter<I>
where
    I: Iterator<Item = (usize, io::Result<String>)>,
{
    iter: I,
    buffer: Vec<<I as Iterator>::Item>,
    /// the number of elements the buffer may hold
    size: usize,
    /// flag to indicate content off the buffer should be returned instead of off the wrapped
    /// iterator
    rewind: bool,
}

impl<I> InputSplitter<I>
where
    I: Iterator<Item = (usize, io::Result<String>)>,
{
    fn new(iter: I) -> InputSplitter<I> {
        InputSplitter {
            iter,
            buffer: Vec::new(),
            rewind: false,
            size: 1,
        }
    }

    /// Rewind the iteration by outputting the buffer's content.
    fn rewind_buffer(&mut self) {
        self.rewind = true;
    }

    /// Shrink the buffer so that its length is equal to the set size, returning an iterator for
    /// the elements that were too much.
    fn shrink_buffer_to_size(&mut self) -> impl Iterator<Item = String> + '_ {
        let mut shrink_offset = 0;
        if self.buffer.len() > self.size {
            shrink_offset = self.buffer.len() - self.size;
        }
        self.buffer
            .drain(..shrink_offset)
            .map(|(_, line)| line.unwrap())
    }

    /// Drain the content of the buffer.
    fn drain_buffer(&mut self) -> impl Iterator<Item = String> + '_ {
        self.buffer.drain(..).map(|(_, line)| line.unwrap())
    }

    /// Set the maximum number of lines to keep.
    fn set_size_of_buffer(&mut self, size: usize) {
        self.size = size;
    }

    /// Add a line to the buffer. If the buffer has [`size`] elements, then its head is removed and
    /// the new line is pushed to the buffer. The removed head is then available in the returned
    /// option.
    fn add_line_to_buffer(&mut self, ln: usize, line: String) -> Option<String> {
        if self.rewind {
            self.buffer.insert(0, (ln, Ok(line)));
            None
        } else if self.buffer.len() >= self.size {
            let (_, head_line) = self.buffer.remove(0);
            self.buffer.push((ln, Ok(line)));
            Some(head_line.unwrap())
        } else {
            self.buffer.push((ln, Ok(line)));
            None
        }
    }

    /// Returns the number of lines stored in the buffer
    fn buffer_len(&self) -> usize {
        self.buffer.len()
    }
}

impl<I> Iterator for InputSplitter<I>
where
    I: Iterator<Item = (usize, io::Result<String>)>,
{
    type Item = <I as Iterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rewind {
            if !self.buffer.is_empty() {
                return Some(self.buffer.remove(0));
            }
            self.rewind = false;
        }
        self.iter.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_splitter() {
        let input = vec![
            Ok(String::from("aaa")),
            Ok(String::from("bbb")),
            Ok(String::from("ccc")),
            Ok(String::from("ddd")),
        ];
        let mut input_splitter = InputSplitter::new(input.into_iter().enumerate());

        input_splitter.set_size_of_buffer(2);
        assert_eq!(input_splitter.buffer_len(), 0);

        match input_splitter.next() {
            Some((0, Ok(line))) => {
                assert_eq!(line, String::from("aaa"));
                assert_eq!(input_splitter.add_line_to_buffer(0, line), None);
                assert_eq!(input_splitter.buffer_len(), 1);
            }
            item @ _ => panic!("wrong item: {:?}", item),
        };

        match input_splitter.next() {
            Some((1, Ok(line))) => {
                assert_eq!(line, String::from("bbb"));
                assert_eq!(input_splitter.add_line_to_buffer(1, line), None);
                assert_eq!(input_splitter.buffer_len(), 2);
            }
            item @ _ => panic!("wrong item: {:?}", item),
        };

        match input_splitter.next() {
            Some((2, Ok(line))) => {
                assert_eq!(line, String::from("ccc"));
                assert_eq!(
                    input_splitter.add_line_to_buffer(2, line),
                    Some(String::from("aaa"))
                );
                assert_eq!(input_splitter.buffer_len(), 2);
            }
            item @ _ => panic!("wrong item: {:?}", item),
        };

        input_splitter.rewind_buffer();

        match input_splitter.next() {
            Some((1, Ok(line))) => {
                assert_eq!(line, String::from("bbb"));
                assert_eq!(input_splitter.buffer_len(), 1);
            }
            item @ _ => panic!("wrong item: {:?}", item),
        };

        match input_splitter.next() {
            Some((2, Ok(line))) => {
                assert_eq!(line, String::from("ccc"));
                assert_eq!(input_splitter.buffer_len(), 0);
            }
            item @ _ => panic!("wrong item: {:?}", item),
        };

        match input_splitter.next() {
            Some((3, Ok(line))) => {
                assert_eq!(line, String::from("ddd"));
                assert_eq!(input_splitter.buffer_len(), 0);
            }
            item @ _ => panic!("wrong item: {:?}", item),
        };

        assert!(input_splitter.next().is_none());
    }

    #[test]
    fn input_splitter_interrupt_rewind() {
        let input = vec![
            Ok(String::from("aaa")),
            Ok(String::from("bbb")),
            Ok(String::from("ccc")),
            Ok(String::from("ddd")),
        ];
        let mut input_splitter = InputSplitter::new(input.into_iter().enumerate());

        input_splitter.set_size_of_buffer(3);
        assert_eq!(input_splitter.buffer_len(), 0);

        match input_splitter.next() {
            Some((0, Ok(line))) => {
                assert_eq!(line, String::from("aaa"));
                assert_eq!(input_splitter.add_line_to_buffer(0, line), None);
                assert_eq!(input_splitter.buffer_len(), 1);
            }
            item @ _ => panic!("wrong item: {:?}", item),
        };

        match input_splitter.next() {
            Some((1, Ok(line))) => {
                assert_eq!(line, String::from("bbb"));
                assert_eq!(input_splitter.add_line_to_buffer(1, line), None);
                assert_eq!(input_splitter.buffer_len(), 2);
            }
            item @ _ => panic!("wrong item: {:?}", item),
        };

        match input_splitter.next() {
            Some((2, Ok(line))) => {
                assert_eq!(line, String::from("ccc"));
                assert_eq!(input_splitter.add_line_to_buffer(2, line), None);
                assert_eq!(input_splitter.buffer_len(), 3);
            }
            item @ _ => panic!("wrong item: {:?}", item),
        };

        input_splitter.rewind_buffer();

        match input_splitter.next() {
            Some((0, Ok(line))) => {
                assert_eq!(line, String::from("aaa"));
                assert_eq!(input_splitter.add_line_to_buffer(0, line), None);
                assert_eq!(input_splitter.buffer_len(), 3);
            }
            item @ _ => panic!("wrong item: {:?}", item),
        };

        match input_splitter.next() {
            Some((0, Ok(line))) => {
                assert_eq!(line, String::from("aaa"));
                assert_eq!(input_splitter.buffer_len(), 2);
            }
            item @ _ => panic!("wrong item: {:?}", item),
        };

        match input_splitter.next() {
            Some((1, Ok(line))) => {
                assert_eq!(line, String::from("bbb"));
                assert_eq!(input_splitter.buffer_len(), 1);
            }
            item @ _ => panic!("wrong item: {:?}", item),
        };

        match input_splitter.next() {
            Some((2, Ok(line))) => {
                assert_eq!(line, String::from("ccc"));
                assert_eq!(input_splitter.buffer_len(), 0);
            }
            item @ _ => panic!("wrong item: {:?}", item),
        };

        match input_splitter.next() {
            Some((3, Ok(line))) => {
                assert_eq!(line, String::from("ddd"));
                assert_eq!(input_splitter.buffer_len(), 0);
            }
            item @ _ => panic!("wrong item: {:?}", item),
        };

        assert!(input_splitter.next().is_none());
    }
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();

    let matches = app!(SYNTAX, SUMMARY, LONG_HELP)
        .optopt(
            "b",
            SUFFIX_FORMAT_OPT,
            "use sprintf FORMAT instead of %02d",
            "FORMAT",
        )
        .optopt("f", PREFIX_OPT, "use PREFIX instead of 'xx'", "PREFIX")
        .optflag("k", KEEP_FILES_OPT, "do not remove output files on errors")
        .optflag(
            "",
            SUPPRESS_MATCHED_OPT,
            "suppress the lines matching PATTERN",
        )
        .optopt(
            "n",
            DIGITS_OPT,
            "use specified number of digits instead of 2",
            "DIGITS",
        )
        .optflag("s", QUIET_OPT, "do not print counts of output file sizes")
        .optflag("z", ELIDE_EMPTY_FILES_OPT, "remove empty output files")
        .parse(args);

    // check for mandatory arguments
    if matches.free.is_empty() {
        show_error!("missing operand");
        exit!(1);
    }
    if matches.free.len() == 1 {
        show_error!("missing operand after '{}'", matches.free[0]);
        exit!(1);
    }
    // get the patterns to split on
    let patterns = return_if_err!(1, patterns::get_patterns(&matches.free[1..]));
    // get the file to split
    let file_name: &str = &matches.free[0];
    let options = CsplitOptions::new(&matches);
    if file_name == "-" {
        let stdin = io::stdin();
        crash_if_err!(1, csplit(&options, patterns, stdin.lock()));
    } else {
        let file = return_if_err!(1, File::open(file_name));
        let file_metadata = return_if_err!(1, file.metadata());
        if !file_metadata.is_file() {
            crash!(1, "'{}' is not a regular file", file_name);
        }
        crash_if_err!(1, csplit(&options, patterns, BufReader::new(file)));
    };
    0
}
