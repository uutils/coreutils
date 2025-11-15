// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore rustdoc
#![allow(rustdoc::private_intra_doc_links)]

use std::cmp::Ordering;
use std::ffi::OsString;
use std::io::{self, BufReader, ErrorKind};
use std::{
    fs::{File, remove_file},
    io::{BufRead, BufWriter, Write},
};

use clap::{Arg, ArgAction, ArgMatches, Command};
use regex::Regex;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult};
use uucore::format_usage;

mod csplit_error;
mod patterns;
mod split_name;

use crate::csplit_error::CsplitError;
use crate::split_name::SplitName;

use uucore::translate;

mod options {
    pub const SUFFIX_FORMAT: &str = "suffix-format";
    pub const SUPPRESS_MATCHED: &str = "suppress-matched";
    pub const DIGITS: &str = "digits";
    pub const PREFIX: &str = "prefix";
    pub const KEEP_FILES: &str = "keep-files";
    pub const QUIET: &str = "quiet";
    pub const ELIDE_EMPTY_FILES: &str = "elide-empty-files";
    pub const FILE: &str = "file";
    pub const PATTERN: &str = "pattern";
}

/// Command line options for csplit.
pub struct CsplitOptions {
    split_name: SplitName,
    keep_files: bool,
    quiet: bool,
    elide_empty_files: bool,
    suppress_matched: bool,
}

impl CsplitOptions {
    fn new(matches: &ArgMatches) -> Result<Self, CsplitError> {
        let keep_files = matches.get_flag(options::KEEP_FILES);
        let quiet = matches.get_flag(options::QUIET);
        let elide_empty_files = matches.get_flag(options::ELIDE_EMPTY_FILES);
        let suppress_matched = matches.get_flag(options::SUPPRESS_MATCHED);

        Ok(Self {
            split_name: SplitName::new(
                matches.get_one::<String>(options::PREFIX).cloned(),
                matches.get_one::<String>(options::SUFFIX_FORMAT).cloned(),
                matches.get_one::<String>(options::DIGITS).cloned(),
            )?,
            keep_files,
            quiet,
            elide_empty_files,
            suppress_matched,
        })
    }
}

pub struct LinesWithNewlines<T: BufRead> {
    inner: T,
}

impl<T: BufRead> LinesWithNewlines<T> {
    fn new(s: T) -> Self {
        Self { inner: s }
    }
}

impl<T: BufRead> Iterator for LinesWithNewlines<T> {
    type Item = io::Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        fn ret(v: Vec<u8>) -> io::Result<String> {
            String::from_utf8(v).map_err(|_| {
                io::Error::new(ErrorKind::InvalidData, translate!("csplit-stream-not-utf8"))
            })
        }

        let mut v = Vec::new();
        match self.inner.read_until(b'\n', &mut v) {
            Ok(0) => None,
            Ok(_) => Some(ret(v)),
            Err(e) => Some(Err(e)),
        }
    }
}

/// Splits a file into severals according to the command line patterns.
///
/// # Errors
///
/// - [`io::Error`] if there is some problem reading/writing from/to a file.
/// - [`CsplitError::LineOutOfRange`] if the line number pattern is larger than the number of input
///   lines.
/// - [`CsplitError::LineOutOfRangeOnRepetition`], like previous but after applying the pattern
///   more than once.
/// - [`CsplitError::MatchNotFound`] if no line matched a regular expression.
/// - [`CsplitError::MatchNotFoundOnRepetition`], like previous but after applying the pattern
///   more than once.
pub fn csplit<T>(options: &CsplitOptions, patterns: &[String], input: T) -> Result<(), CsplitError>
where
    T: BufRead,
{
    let enumerated_input_lines = LinesWithNewlines::new(input)
        .map(|line| line.map_err_context(|| translate!("csplit-read-error")))
        .enumerate();
    let mut input_iter = InputSplitter::new(enumerated_input_lines);
    let mut split_writer = SplitWriter::new(options);
    let patterns_vec: Vec<patterns::Pattern> = patterns::get_patterns(patterns)?;
    let all_up_to_line = patterns_vec
        .iter()
        .all(|p| matches!(p, patterns::Pattern::UpToLine(_, _)));
    let ret = do_csplit(&mut split_writer, patterns_vec, &mut input_iter);

    // consume the rest, unless there was an error
    if ret.is_ok() {
        input_iter.rewind_buffer();
        if let Some((_, line)) = input_iter.next() {
            // There is remaining input: create a final split and copy remainder
            split_writer.new_writer()?;
            split_writer.writeln(&line?)?;
            for (_, line) in input_iter {
                split_writer.writeln(&line?)?;
            }
            split_writer.finish_split();
        } else if all_up_to_line && options.suppress_matched {
            // GNU semantics for integer patterns with --suppress-matched:
            // even if no remaining input, create a final (possibly empty) split
            split_writer.new_writer()?;
            split_writer.finish_split();
        }
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
    I: Iterator<Item = (usize, UResult<String>)>,
{
    // split the file based on patterns
    for pattern in patterns {
        let pattern_as_str = pattern.to_string();
        let is_skip = matches!(pattern, patterns::Pattern::SkipToMatch(_, _, _));
        match pattern {
            patterns::Pattern::UpToLine(n, ex) => {
                let mut up_to_line = n;
                for (_, ith) in ex.iter() {
                    split_writer.new_writer()?;
                    match split_writer.do_to_line(&pattern_as_str, up_to_line, input_iter) {
                        // the error happened when applying the pattern more than once
                        Err(CsplitError::LineOutOfRange(_)) if ith != 1 => {
                            return Err(CsplitError::LineOutOfRangeOnRepetition(
                                pattern_as_str,
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
                                pattern_as_str,
                                ith - 1,
                            ));
                        }
                        (Err(err), _) => return Err(err),
                        // continue the splitting process
                        (Ok(()), _) => (),
                    }
                }
            }
        }
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

impl Drop for SplitWriter<'_> {
    fn drop(&mut self) {
        if self.options.elide_empty_files && self.size == 0 {
            let file_name = self.options.split_name.get(self.counter);
            // In the case of `echo a | csplit -z - %a%1`, the file
            // `xx00` does not exist because the positive offset
            // advanced past the end of the input. Since there is no
            // file to remove in that case, `remove_file` would return
            // an error, so we just ignore it.
            let _ = remove_file(file_name);
        }
    }
}

impl SplitWriter<'_> {
    fn new(options: &CsplitOptions) -> SplitWriter<'_> {
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
        let file = File::create(file_name)?;
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

    /// Writes the line to the current split.
    /// If `self.dev_null` is true, then the line is discarded.
    ///
    /// # Errors
    ///
    /// Some [`io::Error`] may occur when attempting to write the line.
    fn writeln(&mut self, line: &str) -> io::Result<()> {
        if !self.dev_null {
            match self.current_writer {
                Some(ref mut current_writer) => {
                    let bytes = line.as_bytes();
                    current_writer.write_all(bytes)?;
                    self.size += bytes.len();
                }
                None => panic!("{}", translate!("csplit-write-split-not-created")),
            }
        }
        Ok(())
    }

    /// Perform some operations after completing a split, i.e., either remove it
    /// if the [`options::ELIDE_EMPTY_FILES`] option is enabled, or print how much bytes were written
    /// to it if [`options::QUIET`] is disabled.
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
    /// [`CsplitError::LineOutOfRange`] error is returned.
    fn do_to_line<I>(
        &mut self,
        pattern_as_str: &str,
        n: usize,
        input_iter: &mut InputSplitter<I>,
    ) -> Result<(), CsplitError>
    where
        I: Iterator<Item = (usize, UResult<String>)>,
    {
        input_iter.rewind_buffer();
        input_iter.set_size_of_buffer(1);

        let mut ret = Err(CsplitError::LineOutOfRange(pattern_as_str.to_string()));
        while let Some((ln, line)) = input_iter.next() {
            let line = line?;
            match n.cmp(&(&ln + 1)) {
                Ordering::Less => {
                    assert!(
                        input_iter.add_line_to_buffer(ln, line).is_none(),
                        "the buffer is big enough to contain 1 line"
                    );
                    ret = Ok(());
                    break;
                }
                Ordering::Equal => {
                    assert!(
                        self.options.suppress_matched
                            || input_iter.add_line_to_buffer(ln, line).is_none(),
                        "the buffer is big enough to contain 1 line"
                    );
                    ret = Ok(());
                    break;
                }
                Ordering::Greater => (),
            }
            self.writeln(&line)?;
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
    /// - if no line matched, an [`CsplitError::MatchNotFound`].
    /// - if there are not enough lines to accommodate the offset, an
    ///   [`CsplitError::LineOutOfRange`].
    #[allow(clippy::cognitive_complexity)]
    fn do_to_match<I>(
        &mut self,
        pattern_as_str: &str,
        regex: &Regex,
        mut offset: i32,
        input_iter: &mut InputSplitter<I>,
    ) -> Result<(), CsplitError>
    where
        I: Iterator<Item = (usize, UResult<String>)>,
    {
        if offset >= 0 {
            // The offset is zero or positive, no need for a buffer on the lines read.
            // NOTE: drain the buffer of input_iter, no match should be done within.
            for line in input_iter.drain_buffer() {
                self.writeln(&line)?;
            }
            // retain the matching line
            input_iter.set_size_of_buffer(1);

            while let Some((ln, line)) = input_iter.next() {
                let line = line?;
                let l = line
                    .strip_suffix("\r\n")
                    .unwrap_or_else(|| line.strip_suffix('\n').unwrap_or(&line));
                if regex.is_match(l) {
                    let mut next_line_suppress_matched = false;
                    match (self.options.suppress_matched, offset) {
                        // no offset, add the line to the next split
                        (false, 0) => {
                            assert!(
                                input_iter.add_line_to_buffer(ln, line).is_none(),
                                "the buffer is big enough to contain 1 line"
                            );
                        }
                        // a positive offset, some more lines need to be added to the current split
                        (false, _) => self.writeln(&line)?,
                        // suppress matched option true, but there is a positive offset, so the line is printed
                        (true, 1..) => {
                            next_line_suppress_matched = true;
                            self.writeln(&line)?;
                        }
                        _ => (),
                    }
                    offset -= 1;

                    // write the extra lines required by the offset
                    while offset > 0 {
                        match input_iter.next() {
                            Some((_, line)) => {
                                self.writeln(&line?)?;
                            }
                            None => {
                                self.finish_split();
                                return Err(CsplitError::LineOutOfRange(
                                    pattern_as_str.to_string(),
                                ));
                            }
                        }
                        offset -= 1;
                    }
                    self.finish_split();

                    // if we have to suppress one line after we take the next and do nothing
                    if next_line_suppress_matched {
                        input_iter.next();
                    }
                    return Ok(());
                }
                self.writeln(&line)?;
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
                let line = line?;
                let l = line
                    .strip_suffix("\r\n")
                    .unwrap_or_else(|| line.strip_suffix('\n').unwrap_or(&line));
                if regex.is_match(l) {
                    for line in input_iter.shrink_buffer_to_size() {
                        self.writeln(&line)?;
                    }
                    if self.options.suppress_matched {
                        // since offset_usize is for sure greater than 0
                        // the first element of the buffer should be removed and this
                        // line inserted to be coherent with GNU implementation
                        input_iter.add_line_to_buffer(ln, line);
                    } else {
                        // add 1 to the buffer size to make place for the matched line
                        input_iter.set_size_of_buffer(offset_usize + 1);
                        assert!(
                            input_iter.add_line_to_buffer(ln, line).is_none(),
                            "should be big enough to hold every lines"
                        );
                    }

                    self.finish_split();
                    if input_iter.buffer_len() < offset_usize {
                        return Err(CsplitError::LineOutOfRange(pattern_as_str.to_string()));
                    }
                    return Ok(());
                }
                if let Some(line) = input_iter.add_line_to_buffer(ln, line) {
                    self.writeln(&line)?;
                }
            }
            // no match, drain the buffer into the current split
            for line in input_iter.drain_buffer() {
                self.writeln(&line)?;
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
    I: Iterator<Item = (usize, UResult<String>)>,
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
    I: Iterator<Item = (usize, UResult<String>)>,
{
    fn new(iter: I) -> Self {
        Self {
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
        let shrink_offset = if self.buffer.len() > self.size {
            self.buffer.len() - self.size
        } else {
            0
        };
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

    /// Add a line to the buffer. If the buffer has `self.size` elements, then its head is removed and
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
    I: Iterator<Item = (usize, UResult<String>)>,
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

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    // get the file to split
    let file_name = matches.get_one::<OsString>(options::FILE).unwrap();

    // get the patterns to split on
    let patterns: Vec<String> = matches
        .get_many::<String>(options::PATTERN)
        .unwrap()
        .map(ToOwned::to_owned)
        .collect();
    let options = CsplitOptions::new(&matches)?;
    if file_name == "-" {
        let stdin = io::stdin();
        Ok(csplit(&options, &patterns, stdin.lock())?)
    } else {
        let file = File::open(file_name)
            .map_err_context(|| format!("cannot open {} for reading", file_name.quote()))?;
        Ok(csplit(&options, &patterns, BufReader::new(file))?)
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("csplit-about"))
        .override_usage(format_usage(&translate!("csplit-usage")))
        .args_override_self(true)
        .infer_long_args(true)
        .arg(
            Arg::new(options::SUFFIX_FORMAT)
                .short('b')
                .long(options::SUFFIX_FORMAT)
                .value_name("FORMAT")
                .help(translate!("csplit-help-suffix-format")),
        )
        .arg(
            Arg::new(options::PREFIX)
                .short('f')
                .long(options::PREFIX)
                .value_name("PREFIX")
                .help(translate!("csplit-help-prefix")),
        )
        .arg(
            Arg::new(options::KEEP_FILES)
                .short('k')
                .long(options::KEEP_FILES)
                .help(translate!("csplit-help-keep-files"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SUPPRESS_MATCHED)
                .long(options::SUPPRESS_MATCHED)
                .help(translate!("csplit-help-suppress-matched"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DIGITS)
                .short('n')
                .long(options::DIGITS)
                .value_name("DIGITS")
                .help(translate!("csplit-help-digits")),
        )
        .arg(
            Arg::new(options::QUIET)
                .short('q')
                .long(options::QUIET)
                .visible_short_alias('s')
                .visible_alias("silent")
                .help(translate!("csplit-help-quiet"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ELIDE_EMPTY_FILES)
                .short('z')
                .long(options::ELIDE_EMPTY_FILES)
                .help(translate!("csplit-help-elide-empty-files"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .required(true)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::PATTERN)
                .hide(true)
                .action(ArgAction::Append)
                .required(true),
        )
        .after_help(translate!("csplit-after-help"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::cognitive_complexity)]
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
            item => panic!("wrong item: {item:?}"),
        }

        match input_splitter.next() {
            Some((1, Ok(line))) => {
                assert_eq!(line, String::from("bbb"));
                assert_eq!(input_splitter.add_line_to_buffer(1, line), None);
                assert_eq!(input_splitter.buffer_len(), 2);
            }
            item => panic!("wrong item: {item:?}"),
        }

        match input_splitter.next() {
            Some((2, Ok(line))) => {
                assert_eq!(line, String::from("ccc"));
                assert_eq!(
                    input_splitter.add_line_to_buffer(2, line),
                    Some(String::from("aaa"))
                );
                assert_eq!(input_splitter.buffer_len(), 2);
            }
            item => panic!("wrong item: {item:?}"),
        }

        input_splitter.rewind_buffer();

        match input_splitter.next() {
            Some((1, Ok(line))) => {
                assert_eq!(line, String::from("bbb"));
                assert_eq!(input_splitter.buffer_len(), 1);
            }
            item => panic!("wrong item: {item:?}"),
        }

        match input_splitter.next() {
            Some((2, Ok(line))) => {
                assert_eq!(line, String::from("ccc"));
                assert_eq!(input_splitter.buffer_len(), 0);
            }
            item => panic!("wrong item: {item:?}"),
        }

        match input_splitter.next() {
            Some((3, Ok(line))) => {
                assert_eq!(line, String::from("ddd"));
                assert_eq!(input_splitter.buffer_len(), 0);
            }
            item => panic!("wrong item: {item:?}"),
        }

        assert!(input_splitter.next().is_none());
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
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
            item => panic!("wrong item: {item:?}"),
        }

        match input_splitter.next() {
            Some((1, Ok(line))) => {
                assert_eq!(line, String::from("bbb"));
                assert_eq!(input_splitter.add_line_to_buffer(1, line), None);
                assert_eq!(input_splitter.buffer_len(), 2);
            }
            item => panic!("wrong item: {item:?}"),
        }

        match input_splitter.next() {
            Some((2, Ok(line))) => {
                assert_eq!(line, String::from("ccc"));
                assert_eq!(input_splitter.add_line_to_buffer(2, line), None);
                assert_eq!(input_splitter.buffer_len(), 3);
            }
            item => panic!("wrong item: {item:?}"),
        }

        input_splitter.rewind_buffer();

        match input_splitter.next() {
            Some((0, Ok(line))) => {
                assert_eq!(line, String::from("aaa"));
                assert_eq!(input_splitter.add_line_to_buffer(0, line), None);
                assert_eq!(input_splitter.buffer_len(), 3);
            }
            item => panic!("wrong item: {item:?}"),
        }

        match input_splitter.next() {
            Some((0, Ok(line))) => {
                assert_eq!(line, String::from("aaa"));
                assert_eq!(input_splitter.buffer_len(), 2);
            }
            item => panic!("wrong item: {item:?}"),
        }

        match input_splitter.next() {
            Some((1, Ok(line))) => {
                assert_eq!(line, String::from("bbb"));
                assert_eq!(input_splitter.buffer_len(), 1);
            }
            item => panic!("wrong item: {item:?}"),
        }

        match input_splitter.next() {
            Some((2, Ok(line))) => {
                assert_eq!(line, String::from("ccc"));
                assert_eq!(input_splitter.buffer_len(), 0);
            }
            item => panic!("wrong item: {item:?}"),
        }

        match input_splitter.next() {
            Some((3, Ok(line))) => {
                assert_eq!(line, String::from("ddd"));
                assert_eq!(input_splitter.buffer_len(), 0);
            }
            item => panic!("wrong item: {item:?}"),
        }

        assert!(input_splitter.next().is_none());
    }
}
