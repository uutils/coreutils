// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore rustdoc
#![allow(rustdoc::private_intra_doc_links)]

use std::collections::VecDeque;
use std::io::{self, BufReader};
use std::{
    fs::{File, remove_file},
    io::{BufRead, Write},
};

use clap::{Arg, ArgAction, ArgMatches, Command};
use regex::Regex;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult};
use uucore::{format_usage, help_about, help_section, help_usage, show};

mod csplit_error;
mod patterns;
mod split_name;

use crate::csplit_error::CsplitError;
use crate::patterns::{ExecutePattern, Pattern, get_patterns};
use crate::split_name::SplitName;

const ABOUT: &str = help_about!("csplit.md");
const AFTER_HELP: &str = help_section!("after help", "csplit.md");
const USAGE: &str = help_usage!("csplit.md");

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

/// Whether to continue the main csplit loop or terminate immediately.
enum LoopStatus {
    Continue,
    Return,
}

/// The type of line being buffered.
#[allow(clippy::enum_variant_names)]
enum LineType {
    /// A non-matching line that is buffered due to an offset.
    NonMatch,

    /// A line that matched a regular expression pattern.
    RegexMatch,

    /// A line that was at a specific line number.
    LineNumMatch,
}

/// Object that splits a sequence of lines into chunks.
///
/// The `buffer`, `lines_iter`, `current_line`, and `num_chunks` fields
/// represent the global state of the process across all chunks.
///
/// The `total_bytes`, `file_name`, and `file` represent per-chunk
/// state. At the beginning of each chunk, these are reset.
struct Splitter<'a, B> {
    /// Command-line options that control various behaviors.
    options: &'a CsplitOptions,

    /// Buffer used to store matching lines and offset lines across chunks.
    buffer: VecDeque<(String, LineType)>,

    /// Iterator over lines of the input.
    lines_iter: io::Lines<B>,

    /// The current line number, or the number of lines read from the input.
    current_line: usize,

    /// Total number of chunks that have every been made.
    num_chunks: usize,

    /// Total bytes written in this chunk.
    total_bytes: usize,

    /// The name of the file to which lines from this chunk are written.
    file_name: String,

    /// The file to which lines from this chunk are written.
    file: File,
}

impl<'a, B> Splitter<'a, B>
where
    B: BufRead,
{
    fn new(options: &'a CsplitOptions, input: B) -> io::Result<Self> {
        let buffer = VecDeque::new();
        let lines_iter = input.lines();
        let file_name = options.split_name.get(0);
        let file = File::create(&file_name)?;
        Ok(Self {
            options,
            buffer,
            lines_iter,
            current_line: 0,
            num_chunks: 0,
            total_bytes: 0,
            file_name,
            file,
        })
    }

    /// Print each line from the buffer.
    ///
    /// Returns the total bytes written and whether there was a
    /// matching line in the buffer that was suppressed due to the
    /// `suppress_matched` flag.
    fn print_buffer(&mut self) -> io::Result<(usize, bool)> {
        let mut total = 0;
        let mut suppressed_match = false;
        loop {
            match self.buffer.pop_front() {
                None => break,
                Some((_, LineType::LineNumMatch | LineType::RegexMatch))
                    if self.options.suppress_matched =>
                {
                    suppressed_match = true;
                }
                Some((line, _)) => {
                    writeln!(self.file, "{line}")?;
                    total += line.len() + 1;
                }
            }
        }
        Ok((total, suppressed_match))
    }

    /// Print the first `n` lines from the buffer.
    ///
    /// Returns the total number of bytes written.
    fn print_buffer_n(&mut self, n: usize) -> io::Result<usize> {
        let mut total = 0;
        for _ in 0..n {
            match self.buffer.pop_front() {
                None => break,
                Some((_, LineType::LineNumMatch | LineType::RegexMatch))
                    if self.options.suppress_matched =>
                {
                    continue;
                }
                Some((line, _)) => {
                    writeln!(self.file, "{line}")?;
                    total += line.len() + 1;
                }
            }
        }
        Ok(total)
    }

    /// Prepare the global and per-chunk state for the next chunk.
    fn prepare_next_chunk(&mut self) -> io::Result<()> {
        // Global state.
        self.num_chunks += 1;

        // Per-chunk state.
        self.total_bytes = 0;
        self.file_name = self.options.split_name.get(self.num_chunks);
        self.file = File::create(&self.file_name)?;
        Ok(())
    }

    /// Create a new chunk according to the parameters of the given pattern.
    ///
    /// This takes into account the global state, like the current
    /// state of the iterator over the lines of input, and assumes
    /// that `prepare_next_chunk()` has just been called immediately
    /// before.
    fn handle_pattern(&mut self, pattern: Pattern) -> io::Result<LoopStatus> {
        // For the sake brevity, import these names.
        use crate::patterns::ExecutePattern::{Always, Times};
        use crate::patterns::Pattern::{SkipToMatch, UpToLine, UpToMatch};

        // Positive and negative offsets are split out into their own
        // arms because their behaviors are so different.
        match pattern {
            UpToLine(n, Always) => up_to_line_loop(self, Always, n),
            UpToLine(n, Times(k)) => up_to_line_loop(self, Times(k), n),
            UpToMatch(regex, offset @ 0.., Always) => {
                up_to_match_pos_offset_always(self, regex, offset)
            }
            UpToMatch(regex, offset @ ..0, Always) => {
                up_to_match_neg_offset_always(self, regex, offset)
            }
            UpToMatch(regex, offset @ 0.., Times(k)) => {
                up_to_match_pos_offset_repeat(self, regex, offset, k)
            }
            UpToMatch(regex, offset @ ..0, Times(k)) => {
                up_to_match_neg_offset_repeat(self, regex, offset, k)
            }
            SkipToMatch(regex, offset @ 0.., Always) => {
                skip_to_match_pos_offset_always(self, regex, offset)
            }
            SkipToMatch(regex, offset @ ..0, Always) => {
                skip_to_match_neg_offset_always(self, regex, offset)
            }
            SkipToMatch(regex, offset @ 0.., Times(k)) => {
                skip_to_match_pos_offset_repeat(self, regex, offset, k)
            }
            SkipToMatch(regex, offset @ ..0, Times(k)) => {
                skip_to_match_neg_offset_repeat(self, regex, offset, k)
            }
        }
    }
}

/// Create chunks with a specified number of lines from the input.
///
/// Handles patterns like
///
/// ```text
/// 123
/// 123 {45}
/// 123 {*}
/// ```
///
/// `n` is the target line number and `loop_type` indicates whether to
/// loop a finite or infinite number of times.
fn up_to_line_loop<B>(
    splitter: &mut Splitter<B>,
    loop_type: ExecutePattern,
    n: usize,
) -> io::Result<LoopStatus>
where
    B: BufRead,
{
    let mut i = 0;
    while loop_type.should_continue(i) {
        // The target line number for this chunk, relative to the
        // start of this pattern.
        let next_target = n * (i + 1);

        // If the target line number is less than the absolute current
        // line number, then print the buffered lines to get up to the
        // current line. This can happen with `csplit /15/-3 14`, where
        // the buffered lines exceed the target line number.
        if next_target < splitter.current_line {
            let max =
                (splitter.buffer.len() - 1).saturating_sub(splitter.current_line - next_target);
            let num_bytes = splitter.print_buffer_n(max)?;
            splitter.total_bytes += num_bytes;
        } else {
            // If the target line number is beyond the absolute current
            // line number, then print the contents of the buffer.
            if splitter.current_line < next_target {
                let (num_bytes, _) = splitter.print_buffer()?;
                splitter.total_bytes += num_bytes;
            }

            let max = next_target.saturating_sub(splitter.current_line);
            for j in 0..max {
                splitter.current_line += 1;
                match splitter.lines_iter.next() {
                    // If there are no more lines but we expected at least
                    // one more, that's an error.
                    None => {
                        // Print the total number of bytes in this chunk up to this point.
                        if !splitter.options.quiet {
                            println!("{}", splitter.total_bytes);
                        }

                        // Since this is an error situation, remove all files.
                        if !splitter.options.keep_files {
                            for i in 0..=splitter.num_chunks {
                                remove_file(splitter.options.split_name.get(i))?;
                            }
                        }

                        if i == 0 {
                            show!(CsplitError::LineOutOfRange(format!("{n}")));
                        } else {
                            show!(CsplitError::LineOutOfRangeOnRepetition(format!("{n}"), i));
                        }
                        return Ok(LoopStatus::Return);
                    }
                    // Print the current line, if needed.
                    Some(line) => {
                        let line = line?;
                        // The last line is considered the next
                        // matching line, so we add it to the buffer
                        // to get carried over to the next chunk.
                        if j == max - 1 {
                            splitter.buffer.push_back((line, LineType::LineNumMatch));
                            continue;
                        }
                        writeln!(&mut splitter.file, "{line}")?;
                        splitter.total_bytes += line.len() + 1;
                    }
                };
            }
        }

        if !splitter.options.elide_empty_files || splitter.total_bytes > 0 {
            if !splitter.options.quiet {
                println!("{}", splitter.total_bytes);
            }
            splitter.prepare_next_chunk()?;
        }
        i += 1;
    }
    Ok(LoopStatus::Continue)
}

/// Whether to include all lines up to a regular expression or skip them.
enum PatternType {
    /// Include all lines up to a regular expression, as in `/abc/`.
    Keep,

    /// Skip all lines up to a regular expression, as in `%abc%`.
    Skip,
}

/// String representation of the given pattern.
fn to_match_string(pattern_type: PatternType, pattern: Regex, offset: i32) -> String {
    let s = pattern.as_str();
    match (pattern_type, offset) {
        (PatternType::Keep, 0) => format!("/{s}/"),
        (PatternType::Skip, 0) => format!("%{s}%"),
        (PatternType::Keep, offset) => format!("/{s}/{offset:+}"),
        (PatternType::Skip, offset) => format!("%{s}%{offset:+}"),
    }
}

/// Create the appropriate error for the given situation.
///
/// `pattern_type`, `pattern`, and `offset` together specify the
/// pattern. `i` is the loop iteration on which the problem
/// occurred. `offset_remaining` indicates how much of the offset is
/// remaining to be processed, if any.
fn to_match_error(
    pattern: Regex,
    offset: i32,
    i: usize,
    offset_remaining: Option<i32>,
    pattern_type: PatternType,
) -> CsplitError {
    let s = to_match_string(pattern_type, pattern, offset);
    match (i, offset_remaining) {
        (0, Some(_)) => CsplitError::LineOutOfRange(s),
        (0, _) => CsplitError::MatchNotFound(s),
        (_, Some(_)) => CsplitError::LineOutOfRangeOnRepetition(s, i),
        (_, _) => CsplitError::MatchNotFoundOnRepetition(s, i),
    }
}

/// Create chunks up to a line that matches the given pattern.
///
/// Handles patterns like
///
/// ```text
/// /abc/
/// /abc/ {34}
/// /abc/+12
/// /abc/+12 {34}
/// ```
fn up_to_match_pos_offset_repeat<B>(
    splitter: &mut Splitter<B>,
    pattern: Regex,
    offset: i32,
    k: usize,
) -> io::Result<LoopStatus>
where
    B: BufRead,
{
    let mut offset_remaining = None;

    for i in 0..k {
        loop {
            match splitter.buffer.pop_front() {
                None => break,
                Some((_, LineType::LineNumMatch | LineType::RegexMatch))
                    if splitter.options.suppress_matched =>
                {
                    continue;
                }
                Some((line, LineType::LineNumMatch)) if pattern.is_match(&line) => {
                    splitter.buffer.push_front((line, LineType::RegexMatch));
                    if !splitter.options.quiet {
                        println!("{}", splitter.total_bytes);
                    }
                    splitter.prepare_next_chunk()?;
                    return Ok(LoopStatus::Continue);
                }
                Some((line, _)) => {
                    writeln!(splitter.file, "{line}")?;
                    splitter.total_bytes += line.len() + 1;
                }
            }
        }

        loop {
            splitter.current_line += 1;
            match splitter.lines_iter.next() {
                // If there are no more lines but we expected to find
                // another match (or expected more lines due to a
                // positive offset), that's an error.
                None => {
                    // We were in the middle of a chunk but we ran out of lines.
                    if splitter.total_bytes > 0 && !splitter.options.quiet {
                        println!("{}", splitter.total_bytes);
                    }

                    if !splitter.options.keep_files {
                        for i in 0..=splitter.num_chunks {
                            remove_file(splitter.options.split_name.get(i))?;
                        }
                    }

                    show!(to_match_error(
                        pattern,
                        offset,
                        i,
                        offset_remaining,
                        PatternType::Keep
                    ));
                    return Ok(LoopStatus::Return);
                }
                Some(line) => {
                    let line = line?;
                    if pattern.is_match(&line) {
                        offset_remaining = Some(offset);
                    }
                    if let Some(0) = offset_remaining {
                        splitter.buffer.push_back((line, LineType::RegexMatch));
                        break;
                    }
                    offset_remaining = offset_remaining.map(|x| x - 1);
                    writeln!(&mut splitter.file, "{line}")?;
                    splitter.total_bytes += line.len() + 1;
                }
            };
        }
        if !splitter.options.quiet {
            println!("{}", splitter.total_bytes);
        }
        offset_remaining = None;

        splitter.prepare_next_chunk()?;
    }
    Ok(LoopStatus::Continue)
}

/// Create chunks up to a line that matches the given pattern.
///
/// Handles patterns like
///
/// ```text
/// /abc/ {*}
/// /abc/+12 {*}
/// ```
fn up_to_match_pos_offset_always<B>(
    splitter: &mut Splitter<B>,
    pattern: Regex,
    offset: i32,
) -> io::Result<LoopStatus>
where
    B: BufRead,
{
    let mut offset_remaining = None;

    loop {
        let (num_bytes, suppressed_match) = splitter.print_buffer()?;
        splitter.total_bytes += num_bytes;

        loop {
            splitter.current_line += 1;
            match splitter.lines_iter.next() {
                // If there are no more lines, then we are done.
                None => {
                    // Here we handle two situations,
                    //
                    //     seq 10 | csplit - /bogus/ {*}
                    //     seq 10 | csplit --suppress-matched - /0$/ {*}
                    //
                    // In the first, there was no match anywhere. In
                    // the second, the last match is at the end of the
                    // file and it was suppressed. In the latter case
                    // we still need to print the 0 representing the
                    // total bytes in the last chunk.
                    if splitter.total_bytes == 0 {
                        if splitter.options.elide_empty_files {
                            remove_file(&splitter.file_name)?;
                        } else if suppressed_match && !splitter.options.quiet {
                            println!("{}", splitter.total_bytes);
                        }
                    } else {
                        // Otherwise, just write the total number of
                        // bytes written in this last chunk.
                        if !splitter.options.quiet {
                            println!("{}", splitter.total_bytes);
                        }
                    }
                    return Ok(LoopStatus::Return);
                }
                Some(line) => {
                    let line = line?;
                    if pattern.is_match(&line) {
                        offset_remaining = Some(offset);
                    }
                    if let Some(0) = offset_remaining {
                        splitter.buffer.push_back((line, LineType::RegexMatch));
                        break;
                    }
                    offset_remaining = offset_remaining.map(|x| x - 1);
                    writeln!(&mut splitter.file, "{line}")?;
                    splitter.total_bytes += line.len() + 1;
                }
            };
        }
        if !splitter.options.quiet {
            println!("{}", splitter.total_bytes);
        }
        offset_remaining = None;

        splitter.prepare_next_chunk()?;
    }
}

/// Create chunks up to a line that matches the given pattern.
///
/// Handles patterns like
///
/// ```text
/// /abc/-12
/// /abc/-12 {34}
/// ```
fn up_to_match_neg_offset_repeat<B>(
    splitter: &mut Splitter<B>,
    pattern: Regex,
    offset: i32,
    k: usize,
) -> io::Result<LoopStatus>
where
    B: BufRead,
{
    for i in 0..k {
        loop {
            splitter.current_line += 1;
            match splitter.lines_iter.next() {
                None => {
                    let (num_bytes, _) = splitter.print_buffer()?;
                    splitter.total_bytes += num_bytes;

                    // We were in the middle of a chunk but we ran out of lines.
                    if splitter.total_bytes > 0 && !splitter.options.quiet {
                        println!("{}", splitter.total_bytes);
                    }

                    if !splitter.options.keep_files {
                        for i in 0..=splitter.num_chunks {
                            remove_file(splitter.options.split_name.get(i))?;
                        }
                    }

                    let err = to_match_error(pattern, offset, i, None, PatternType::Keep);
                    show!(err);

                    return Ok(LoopStatus::Return);
                }
                Some(line) => {
                    let line = line?;
                    let is_match = pattern.is_match(&line);
                    if is_match {
                        if splitter.buffer.len() < -offset as usize {
                            splitter.total_bytes = 0;
                            println!("{}", splitter.total_bytes);

                            if !splitter.options.keep_files {
                                for i in 0..=splitter.num_chunks {
                                    remove_file(splitter.options.split_name.get(i))?;
                                }
                            }

                            let err =
                                to_match_error(pattern, offset, i, Some(0), PatternType::Keep);
                            show!(err);

                            return Ok(LoopStatus::Return);
                        } else {
                            let num_lines = splitter.buffer.len() - (-offset) as usize;
                            let num_bytes = splitter.print_buffer_n(num_lines)?;
                            splitter.total_bytes += num_bytes;

                            splitter.buffer.push_back((line, LineType::NonMatch));
                            match splitter.buffer.front_mut() {
                                None => unreachable!(),
                                Some(x) => *x = (x.0.clone(), LineType::RegexMatch),
                            }
                            break;
                        }
                    }
                    if splitter.buffer.len() < -offset as usize {
                        splitter.buffer.push_back((line, LineType::NonMatch));
                    } else {
                        splitter.buffer.push_back((line, LineType::NonMatch));
                        match splitter.buffer.pop_front() {
                            None => unreachable!(),
                            Some((_, LineType::LineNumMatch | LineType::RegexMatch))
                                if splitter.options.suppress_matched =>
                            {
                                continue;
                            }
                            Some((line, _)) => {
                                writeln!(splitter.file, "{line}")?;
                                splitter.total_bytes += line.len() + 1;
                            }
                        }
                    }
                }
            };
        }
        if !splitter.options.quiet {
            println!("{}", splitter.total_bytes);
        }

        splitter.prepare_next_chunk()?;
    }
    Ok(LoopStatus::Continue)
}

/// Create chunks up to a line that matches the given pattern.
///
/// Handles patterns like
///
/// ```text
/// /abc/-12 {*}
/// ```
fn up_to_match_neg_offset_always<B>(
    splitter: &mut Splitter<B>,
    pattern: Regex,
    offset: i32,
) -> io::Result<LoopStatus>
where
    B: BufRead,
{
    loop {
        let (num_bytes, _) = splitter.print_buffer()?;
        splitter.total_bytes += num_bytes;

        loop {
            splitter.current_line += 1;
            match splitter.lines_iter.next() {
                None => {
                    // For example, if the last match was
                    // at the end of the file.
                    if splitter.total_bytes == 0 {
                        if splitter.options.elide_empty_files {
                            remove_file(&splitter.file_name)?;
                        }
                    } else if !splitter.options.quiet {
                        println!("{}", splitter.total_bytes);
                    }
                    return Ok(LoopStatus::Return);
                }
                Some(line) => {
                    let line = line?;
                    let is_match = pattern.is_match(&line);
                    if is_match {
                        splitter.buffer.push_back((line, LineType::RegexMatch));
                        break;
                    }
                    if splitter.buffer.len() < -offset as usize {
                        splitter.buffer.push_back((line, LineType::NonMatch));
                    } else {
                        splitter.buffer.push_back((line, LineType::NonMatch));
                        match splitter.buffer.pop_front() {
                            None => unreachable!(),
                            Some((_, LineType::LineNumMatch | LineType::RegexMatch))
                                if splitter.options.suppress_matched =>
                            {
                                continue;
                            }
                            Some((line, _)) => {
                                writeln!(splitter.file, "{line}")?;
                                splitter.total_bytes += line.len() + 1;
                            }
                        }
                    }
                }
            };
        }
        if !splitter.options.elide_empty_files || !splitter.total_bytes == 0 {
            if !splitter.options.quiet {
                println!("{}", splitter.total_bytes);
            }

            splitter.prepare_next_chunk()?;
        }
    }
}

/// Skip chunks up to a line that matches the given pattern.
///
/// Handles patterns like
///
/// ```text
/// %abc%
/// %abc%+12
/// %abc% {34}
/// %abc%+12 {34}
/// ```
fn skip_to_match_pos_offset_repeat<B>(
    splitter: &mut Splitter<B>,
    pattern: Regex,
    offset: i32,
    k: usize,
) -> io::Result<LoopStatus>
where
    B: BufRead,
{
    let mut offset_remaining = None;

    for i in 0..k {
        splitter.buffer.clear();

        loop {
            splitter.current_line += 1;
            match splitter.lines_iter.next() {
                None => {
                    // This is okay
                    if offset == 1 {
                        if !splitter.options.keep_files {
                            for i in 0..=splitter.num_chunks {
                                remove_file(splitter.options.split_name.get(i))?;
                            }
                        }

                        return Ok(LoopStatus::Return);
                    }

                    let err =
                        to_match_error(pattern, offset, i, offset_remaining, PatternType::Skip);
                    show!(&err);

                    if !splitter.options.keep_files {
                        for i in 0..splitter.num_chunks {
                            remove_file(splitter.options.split_name.get(i))?;
                        }
                    }

                    // Special case: `seq 50 | csplit -k - %45%+10`:
                    // don't keep the last chunk.
                    if !splitter.options.keep_files
                        || matches!(
                            err,
                            CsplitError::LineOutOfRange(_)
                                | CsplitError::LineOutOfRangeOnRepetition(_, _)
                        )
                    {
                        let i = splitter.num_chunks;
                        remove_file(splitter.options.split_name.get(i))?;
                    }

                    return Ok(LoopStatus::Return);
                }
                Some(line) => {
                    let line = line?;
                    if pattern.is_match(&line) {
                        offset_remaining = Some(offset);
                    }
                    if let Some(0) = offset_remaining {
                        splitter.buffer.push_back((line, LineType::RegexMatch));
                        break;
                    }
                    offset_remaining = offset_remaining.map(|x| x - 1);
                }
            };
        }
        offset_remaining = None;
    }
    Ok(LoopStatus::Continue)
}

/// Skip chunks up to a line that matches the given pattern.
///
/// Handles patterns like
///
/// ```text
/// %abc%-12
/// %abc%-12 {34}
/// ```
fn skip_to_match_neg_offset_repeat<B>(
    splitter: &mut Splitter<B>,
    pattern: Regex,
    offset: i32,
    k: usize,
) -> io::Result<LoopStatus>
where
    B: BufRead,
{
    for i in 0..k {
        splitter.buffer.clear();

        loop {
            splitter.current_line += 1;
            match splitter.lines_iter.next() {
                None => {
                    if !splitter.options.keep_files {
                        for i in 0..=splitter.num_chunks {
                            remove_file(splitter.options.split_name.get(i))?;
                        }
                    }
                    let err = to_match_error(pattern, offset, i, None, PatternType::Skip);
                    show!(err);
                    return Ok(LoopStatus::Return);
                }
                Some(line) => {
                    let line = line?;
                    let is_match = pattern.is_match(&line);
                    if is_match {
                        if splitter.current_line < -offset as usize {
                            let s = to_match_string(PatternType::Skip, pattern, offset);
                            let err = CsplitError::LineOutOfRange(s);
                            show!(&err);

                            if !splitter.options.keep_files {
                                for i in 0..splitter.num_chunks {
                                    remove_file(splitter.options.split_name.get(i))?;
                                }
                            }

                            // Special case: `seq 50 | csplit -k - %5%-10`:
                            // don't keep the last chunk.
                            let i = splitter.num_chunks;
                            remove_file(splitter.options.split_name.get(i))?;

                            return Ok(LoopStatus::Return);
                        }
                        splitter.buffer.push_back((line, LineType::RegexMatch));
                        break;
                    }
                    if splitter.buffer.len() < -offset as usize {
                        splitter.buffer.push_back((line, LineType::NonMatch));
                    } else {
                        splitter.buffer.push_back((line, LineType::NonMatch));
                        splitter.buffer.pop_front().unwrap();
                    }
                }
            };
        }
    }
    Ok(LoopStatus::Continue)
}

/// Skip chunks up to a line that matches the given pattern.
///
/// Handles patterns like
///
/// ```text
/// %abc% {*}
/// %abc%+12 {*}
/// ```
fn skip_to_match_pos_offset_always<B>(
    splitter: &mut Splitter<B>,
    pattern: Regex,
    offset: i32,
) -> io::Result<LoopStatus>
where
    B: BufRead,
{
    let mut offset_remaining = None;
    loop {
        splitter.buffer.clear();
        loop {
            splitter.current_line += 1;
            match splitter.lines_iter.next() {
                None => {
                    remove_file(&splitter.file_name)?;
                    return Ok(LoopStatus::Return);
                }
                Some(line) => {
                    let line = line?;
                    if pattern.is_match(&line) {
                        offset_remaining = Some(offset);
                    }
                    if let Some(0) = offset_remaining {
                        splitter.buffer.push_back((line, LineType::RegexMatch));
                        break;
                    }
                    offset_remaining = offset_remaining.map(|x| x - 1);
                }
            };
        }
        offset_remaining = None;
    }
}

/// Skip chunks up to a line that matches the given pattern.
///
/// Handles patterns like
///
/// ```text
/// %abc%-12 {*}
/// ```
fn skip_to_match_neg_offset_always<B>(
    splitter: &mut Splitter<B>,
    pattern: Regex,
    offset: i32,
) -> io::Result<LoopStatus>
where
    B: BufRead,
{
    loop {
        splitter.buffer.clear();

        loop {
            splitter.current_line += 1;
            match splitter.lines_iter.next() {
                None => {
                    splitter.buffer.clear();
                    remove_file(&splitter.file_name)?;
                    return Ok(LoopStatus::Return);
                }
                Some(line) => {
                    let line = line?;
                    let is_match = pattern.is_match(&line);
                    if is_match {
                        splitter.buffer.push_back((line, LineType::RegexMatch));
                        break;
                    }
                    if splitter.buffer.len() < -offset as usize {
                        splitter.buffer.push_back((line, LineType::NonMatch));
                    } else {
                        splitter.buffer.push_back((line, LineType::NonMatch));
                        splitter.buffer.pop_front().unwrap();
                    }
                }
            };
        }
    }
}

/// Create the final chunk with any remaining lines of the input.
///
/// This is called after all patterns specified on the command line
/// have been processed if they didn't cover the entire input file.
fn up_to_end<B>(splitter: &mut Splitter<B>) -> io::Result<()>
where
    B: BufRead,
{
    // Print the contents of the buffer.
    // let mut total_bytes = 0;
    let (num_bytes, _) = splitter.print_buffer()?;
    splitter.total_bytes += num_bytes;

    loop {
        splitter.current_line += 1;
        match splitter.lines_iter.next() {
            // If there are no more lines, we are done.
            None => {
                if !splitter.options.quiet {
                    println!("{}", splitter.total_bytes);
                }
                return Ok(());
            }
            Some(line) => {
                let line = line?;
                writeln!(&mut splitter.file, "{line}")?;
                splitter.total_bytes += line.len() + 1;
            }
        };
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
    let mut splitter = Splitter::new(options, input)?;
    for pattern in get_patterns(patterns)? {
        match splitter
            .handle_pattern(pattern)
            .map_err_context(|| "read error".to_string())
        {
            Ok(LoopStatus::Continue) => continue,
            Ok(LoopStatus::Return) => return Ok(()),
            Err(e) => {
                if !splitter.options.quiet {
                    println!("{}", splitter.total_bytes);
                }
                return Err(e.into());
            }
        }
    }

    // Write any remaining lines in the buffer and any remaining lines
    // from the input to one final chunk.
    up_to_end(&mut splitter).map_err_context(|| "read error".to_string())?;

    Ok(())
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    // get the file to split
    let file_name = matches.get_one::<String>(options::FILE).unwrap();

    // get the patterns to split on
    let patterns: Vec<String> = matches
        .get_many::<String>(options::PATTERN)
        .unwrap()
        .map(|s| s.to_string())
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
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .args_override_self(true)
        .infer_long_args(true)
        .arg(
            Arg::new(options::SUFFIX_FORMAT)
                .short('b')
                .long(options::SUFFIX_FORMAT)
                .value_name("FORMAT")
                .help("use sprintf FORMAT instead of %02d"),
        )
        .arg(
            Arg::new(options::PREFIX)
                .short('f')
                .long(options::PREFIX)
                .value_name("PREFIX")
                .help("use PREFIX instead of 'xx'"),
        )
        .arg(
            Arg::new(options::KEEP_FILES)
                .short('k')
                .long(options::KEEP_FILES)
                .help("do not remove output files on errors")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SUPPRESS_MATCHED)
                .long(options::SUPPRESS_MATCHED)
                .help("suppress the lines matching PATTERN")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DIGITS)
                .short('n')
                .long(options::DIGITS)
                .value_name("DIGITS")
                .help("use specified number of digits instead of 2"),
        )
        .arg(
            Arg::new(options::QUIET)
                .short('q')
                .long(options::QUIET)
                .visible_short_alias('s')
                .visible_alias("silent")
                .help("do not print counts of output file sizes")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ELIDE_EMPTY_FILES)
                .short('z')
                .long(options::ELIDE_EMPTY_FILES)
                .help("remove empty output files")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .required(true)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::PATTERN)
                .hide(true)
                .action(ArgAction::Append)
                .required(true),
        )
        .after_help(AFTER_HELP)
}
