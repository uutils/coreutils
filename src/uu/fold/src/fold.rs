// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDOs) ncount routput

use clap::{Arg, ArgAction, Command};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write, stdin, stdout};
use std::path::Path;
use unicode_width::UnicodeWidthChar;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::format_usage;
use uucore::show;
use uucore::translate;

const TAB_WIDTH: usize = 8;
const NL: u8 = b'\n';
const CR: u8 = b'\r';
const TAB: u8 = b'\t';
// Implementation threshold (8 KiB) to prevent unbounded buffer growth during streaming.
// Chosen as a small, fixed cap: large enough to avoid excessive flushes, but
// small enough to keep memory bounded when the input has no fold points.
const STREAMING_FLUSH_THRESHOLD: usize = 8 * 1024;

mod options {
    pub const BYTES: &str = "bytes";
    pub const CHARACTERS: &str = "characters";
    pub const SPACES: &str = "spaces";
    pub const PUNCTUATION: &str = "punctuation";
    pub const WIDTH: &str = "width";
    pub const FILE: &str = "file";
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WidthMode {
    Columns,
    Characters,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BreakMode {
    Hard,
    Spaces,
    Punctuation,
}

struct FoldContext<'a, W: Write> {
    break_mode: BreakMode,
    width: usize,
    mode: WidthMode,
    writer: &'a mut W,
    output: &'a mut Vec<u8>,
    col_count: &'a mut usize,
    last_space: &'a mut Option<usize>,
    last_sentence_end: &'a mut Option<usize>,
    last_clause_end: &'a mut Option<usize>,
    last_non_alnum: &'a mut Option<usize>,
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_lossy();

    let (args, obs_width) = handle_obsolete(&args[..]);
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let bytes = matches.get_flag(options::BYTES);
    let characters = matches.get_flag(options::CHARACTERS);
    let spaces = matches.get_flag(options::SPACES);
    let punctuation = matches.get_flag(options::PUNCTUATION);
    let break_mode = if punctuation {
        BreakMode::Punctuation
    } else if spaces {
        BreakMode::Spaces
    } else {
        BreakMode::Hard
    };
    let poss_width = match matches.get_one::<String>(options::WIDTH) {
        Some(v) => Some(v.clone()),
        None => obs_width,
    };

    let width = match poss_width {
        Some(inp_width) => match inp_width.parse::<usize>() {
            Ok(0) => {
                return Err(USimpleError::new(
                    1,
                    translate!("fold-error-illegal-width", "width" => inp_width.quote()),
                ));
            }
            Ok(parsed_width) => parsed_width,
            Err(e) => {
                return Err(USimpleError::new(
                    1,
                    translate!("fold-error-illegal-width", "width" => inp_width.quote(), "error" => e),
                ));
            }
        },
        None => 80,
    };

    let files = match matches.get_many::<String>(options::FILE) {
        Some(v) => v.cloned().collect(),
        None => vec!["-".to_owned()],
    };

    fold(&files, bytes, characters, break_mode, width)
}

pub fn uu_app() -> Command {
    Command::new("fold")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(format_usage(&translate!("fold-usage")))
        .about(translate!("fold-about"))
        .infer_long_args(true)
        .arg(
            Arg::new(options::BYTES)
                .long(options::BYTES)
                .short('b')
                .help(translate!("fold-bytes-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CHARACTERS)
                .long(options::CHARACTERS)
                .short('c')
                .help(translate!("fold-characters-help"))
                .conflicts_with(options::BYTES)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SPACES)
                .long(options::SPACES)
                .short('s')
                .help(translate!("fold-spaces-help"))
                .conflicts_with(options::PUNCTUATION)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PUNCTUATION)
                .long(options::PUNCTUATION)
                .short('p')
                .help("break at punctuation boundaries (tiered: sentence-end, clause, space, non-alphanumeric)")
                .conflicts_with(options::SPACES)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::WIDTH)
                .long(options::WIDTH)
                .short('w')
                .help(translate!("fold-width-help"))
                .value_name("WIDTH")
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
}

fn handle_obsolete(args: &[String]) -> (Vec<String>, Option<String>) {
    for (i, arg) in args.iter().enumerate() {
        let slice = &arg;
        if slice.starts_with('-') && slice.chars().nth(1).is_some_and(|c| c.is_ascii_digit()) {
            let mut v = args.to_vec();
            v.remove(i);
            return (v, Some(slice[1..].to_owned()));
        }
    }
    (args.to_vec(), None)
}

fn fold(
    filenames: &[String],
    bytes: bool,
    characters: bool,
    break_mode: BreakMode,
    width: usize,
) -> UResult<()> {
    let mut output = BufWriter::new(stdout());

    for filename in filenames {
        let filename: &str = filename;
        let mut stdin_buf;
        let mut file_buf;
        let buffer = BufReader::new(if filename == "-" {
            stdin_buf = stdin();
            &mut stdin_buf as &mut dyn Read
        } else {
            // Like GNU, report the error but keep processing the remaining files.
            match File::open(Path::new(filename)) {
                Ok(f) => file_buf = f,
                Err(e) => {
                    show!(e.map_err_context(|| filename.to_string()));
                    continue;
                }
            }
            &mut file_buf as &mut dyn Read
        });

        if bytes {
            fold_file_bytewise(buffer, break_mode, width, &mut output)?;
        } else {
            let mode = if characters {
                WidthMode::Characters
            } else {
                WidthMode::Columns
            };
            fold_file(buffer, break_mode, width, mode, &mut output)?;
        }
    }

    output
        .flush()
        .map_err_context(|| translate!("fold-error-failed-to-write"))?;
    Ok(())
}

fn find_punctuation_break(chunk: &[u8]) -> Option<usize> {
    if let Some(pos) = chunk.iter().rposition(|&b| matches!(b, b'.' | b'!' | b'?' | b':')) {
        return Some(pos + 1);
    }
    if let Some(pos) = chunk.iter().rposition(|&b| matches!(b, b',' | b';')) {
        return Some(pos + 1);
    }
    if let Some(pos) = chunk
        .iter()
        .rposition(|b| b.is_ascii_whitespace() && *b != CR)
    {
        return Some(pos + 1);
    }
    if let Some(pos) = chunk
        .iter()
        .rposition(|b| !b.is_ascii_alphanumeric() && *b != CR)
    {
        return Some(pos + 1);
    }
    None
}

/// Fold `file` to fit `width` (number of columns), counting all characters as
/// one column.
///
/// This function handles folding for the `-b`/`--bytes` option, counting
/// tab, backspace, and carriage return as occupying one column, identically
/// to all other characters in the stream.
///
/// The `break_mode` controls where lines are broken: at whitespace (`Spaces`),
/// at tiered punctuation boundaries (`Punctuation`), or at the hard width limit.
fn fold_file_bytewise<T: Read, W: Write>(
    mut file: BufReader<T>,
    break_mode: BreakMode,
    width: usize,
    output: &mut W,
) -> UResult<()> {
    let mut line = Vec::new();

    loop {
        // Pull bytes from the reader until we have strictly more than `width`
        // buffered (enough to know whether content follows a width-driven fold)
        // or we reach EOF.
        while line.len() <= width {
            let buf = file
                .fill_buf()
                .map_err_context(|| translate!("fold-error-readline"))?;
            if buf.is_empty() {
                break;
            }
            let len = buf.len();
            line.extend_from_slice(buf);
            file.consume(len);
        }

        // EOF with a tail shorter than (or equal to) `width`: no fold can
        // apply, so emit it verbatim (newlines inside are preserved).
        if line.len() <= width {
            if line.is_empty() {
                break;
            }
            output.write_all(&line)?;
            break;
        }

        // We have a full `width`-byte chunk plus at least one lookahead byte.
        let chunk = &line[..width];

        // An existing newline within the chunk ends the line naturally.
        if let Some(end) = chunk.iter().position(|c| *c == NL).map(|i| i + 1) {
            output.write_all(&line[..end])?;
            line.drain(..end);
            continue;
        }

        // No newline found: select break point based on break_mode.
        // With -p, use tiered punctuation search; with -s, break at
        // last whitespace; otherwise hard-wrap at `width`.
        let end = match break_mode {
            BreakMode::Punctuation => {
                find_punctuation_break(chunk).unwrap_or(width)
            }
            BreakMode::Spaces => chunk
                .iter()
                .rposition(|c| c.is_ascii_whitespace() && *c != CR)
                .map_or(width, |i| i + 1),
            BreakMode::Hard => width,
        };

        output.write_all(&line[..end])?;
        // Insert a newline unless the next byte is already a newline.
        if line[end] != NL {
            output.write_all(&[NL])?;
        }
        line.drain(..end);
    }
    Ok(())
}

fn next_tab_stop(col_count: usize) -> usize {
    col_count + TAB_WIDTH - col_count % TAB_WIDTH
}

fn compute_col_count(buffer: &[u8], mode: WidthMode) -> usize {
    if let Ok(s) = std::str::from_utf8(buffer) {
        let mut width = 0;
        for ch in s.chars() {
            match ch {
                '\r' => width = 0,
                '\t' => width = next_tab_stop(width),
                '\x08' => width = width.saturating_sub(1),
                _ => {
                    width += match mode {
                        WidthMode::Characters => 1,
                        WidthMode::Columns => UnicodeWidthChar::width(ch).unwrap_or(0),
                    }
                }
            }
        }
        width
    } else {
        let mut width = 0;
        for &byte in buffer {
            match byte {
                CR => width = 0,
                TAB => width = next_tab_stop(width),
                0x08 => width = width.saturating_sub(1),
                _ => width += 1,
            }
        }
        width
    }
}

fn rebase_tracker(tracker: &mut Option<usize>, consume: usize) {
    *tracker = tracker.and_then(|idx| {
        if idx < consume {
            None
        } else {
            Some(idx - consume)
        }
    });
}

/// Emit one folded line. The break point is selected based on `break_mode`:
/// - `Punctuation`: highest-priority tracked position (sentence > clause > space > non-alnum)
/// - `Spaces`: last remembered whitespace position
/// - `Hard`: current buffer end
///
/// The remainder (if any) stays in the buffer for the next line.
fn emit_output<W: Write>(ctx: &mut FoldContext<'_, W>) -> UResult<()> {
    let consume = match ctx.break_mode {
        BreakMode::Punctuation => ctx
            .last_sentence_end
            .map(|i| i + 1)
            .or_else(|| ctx.last_clause_end.map(|i| i + 1))
            .or_else(|| ctx.last_space.map(|i| i + 1))
            .or_else(|| ctx.last_non_alnum.map(|i| i + 1))
            .unwrap_or(ctx.output.len()),
        BreakMode::Spaces => match *ctx.last_space {
            Some(index) => index + 1,
            None => ctx.output.len(),
        },
        BreakMode::Hard => ctx.output.len(),
    };

    if consume > 0 {
        ctx.writer.write_all(&ctx.output[..consume])?;
    }
    ctx.writer.write_all(&[NL])?;

    if consume < ctx.output.len() {
        ctx.output.drain(..consume);
    } else {
        ctx.output.clear();
    }

    *ctx.col_count = compute_col_count(ctx.output, ctx.mode);

    // Rebase remembered break positions into the remaining buffer.
    match ctx.break_mode {
        BreakMode::Punctuation => {
            rebase_tracker(ctx.last_sentence_end, consume);
            rebase_tracker(ctx.last_clause_end, consume);
            rebase_tracker(ctx.last_space, consume);
            rebase_tracker(ctx.last_non_alnum, consume);
        }
        BreakMode::Spaces => {
            rebase_tracker(ctx.last_space, consume);
        }
        BreakMode::Hard => {
            *ctx.last_space = None;
        }
    }

    Ok(())
}

/// In streaming mode with hard breaks only, avoid unbounded buffering by
/// periodically flushing long unbroken segments. With `-s` or `-p` we must
/// keep the buffer so we can still break at the last tracked boundary.
fn maybe_flush_unbroken_output<W: Write>(ctx: &mut FoldContext<'_, W>) -> UResult<()> {
    if ctx.break_mode != BreakMode::Hard || ctx.output.len() < STREAMING_FLUSH_THRESHOLD {
        return Ok(());
    }

    ctx.writer.write_all(ctx.output)?;
    ctx.output.clear();
    Ok(())
}

fn push_byte<W: Write>(ctx: &mut FoldContext<'_, W>, byte: u8) -> UResult<()> {
    // Append a single byte to the buffer.
    ctx.output.push(byte);
    maybe_flush_unbroken_output(ctx)
}

fn push_bytes<W: Write>(ctx: &mut FoldContext<'_, W>, bytes: &[u8]) -> UResult<()> {
    // Append a byte slice to the buffer and flush if it grows too large.
    if bytes.is_empty() {
        return Ok(());
    }
    ctx.output.extend_from_slice(bytes);
    maybe_flush_unbroken_output(ctx)
}

fn process_ascii_line<W: Write>(line: &[u8], ctx: &mut FoldContext<'_, W>) -> UResult<()> {
    let mut idx = 0;
    let len = line.len();

    while idx < len {
        match line[idx] {
            NL => {
                *ctx.last_space = None;
                emit_output(ctx)?;
                idx += 1;
            }
            CR => {
                push_byte(ctx, CR)?;
                *ctx.col_count = 0;
                idx += 1;
            }
            0x08 => {
                push_byte(ctx, 0x08)?;
                *ctx.col_count = ctx.col_count.saturating_sub(1);
                idx += 1;
            }
            TAB => {
                loop {
                    let next_stop = next_tab_stop(*ctx.col_count);
                    if next_stop > ctx.width && !ctx.output.is_empty() {
                        emit_output(ctx)?;
                        continue;
                    }
                    *ctx.col_count = next_stop;
                    break;
                }
                if ctx.break_mode == BreakMode::Hard {
                    *ctx.last_space = None;
                } else {
                    *ctx.last_space = Some(ctx.output.len());
                }
                push_byte(ctx, TAB)?;
                idx += 1;
            }
            0x00..=0x07 | 0x0B..=0x0C | 0x0E..=0x1F | 0x7F => {
                push_byte(ctx, line[idx])?;
                if ctx.break_mode != BreakMode::Hard
                    && line[idx].is_ascii_whitespace()
                    && line[idx] != CR
                {
                    *ctx.last_space = Some(ctx.output.len() - 1);
                } else if ctx.break_mode == BreakMode::Hard {
                    *ctx.last_space = None;
                }

                if ctx.mode == WidthMode::Characters {
                    *ctx.col_count = ctx.col_count.saturating_add(1);
                    if *ctx.col_count >= ctx.width {
                        emit_output(ctx)?;
                    }
                }
                idx += 1;
            }
            _ => {
                let start = idx;
                while idx < len
                    && !matches!(
                        line[idx],
                        NL | CR | TAB | 0x08 | 0x00..=0x07 | 0x0B..=0x0C | 0x0E..=0x1F | 0x7F
                    )
                {
                    idx += 1;
                }
                push_ascii_segment(&line[start..idx], ctx)?;
            }
        }
    }

    Ok(())
}

fn push_ascii_segment<W: Write>(segment: &[u8], ctx: &mut FoldContext<'_, W>) -> UResult<()> {
    if segment.is_empty() {
        return Ok(());
    }

    let mut remaining = segment;

    while !remaining.is_empty() {
        if *ctx.col_count >= ctx.width {
            emit_output(ctx)?;
            continue;
        }

        let available = ctx.width - *ctx.col_count;
        let take = remaining.len().min(available);
        let base_len = ctx.output.len();

        push_bytes(ctx, &remaining[..take])?;
        *ctx.col_count += take;

        match ctx.break_mode {
            BreakMode::Punctuation => {
                for (i, &b) in remaining[..take].iter().enumerate() {
                    let pos = base_len + i;
                    match b {
                        b'.' | b'!' | b'?' | b':' => {
                            *ctx.last_sentence_end = Some(pos);
                        }
                        b',' | b';' => {
                            *ctx.last_clause_end = Some(pos);
                        }
                        b if b.is_ascii_whitespace() && b != CR => {
                            *ctx.last_space = Some(pos);
                        }
                        b if !b.is_ascii_alphanumeric() => {
                            *ctx.last_non_alnum = Some(pos);
                        }
                        _ => {}
                    }
                }
            }
            BreakMode::Spaces => {
                if let Some(pos) = remaining[..take]
                    .iter()
                    .rposition(|b| b.is_ascii_whitespace() && *b != CR)
                {
                    *ctx.last_space = Some(base_len + pos);
                }
            }
            BreakMode::Hard => {
                *ctx.last_space = None;
            }
        }

        remaining = &remaining[take..];
    }

    Ok(())
}

fn process_utf8_line<W: Write>(line: &str, ctx: &mut FoldContext<'_, W>) -> UResult<()> {
    if line.is_ascii() {
        return process_ascii_line(line.as_bytes(), ctx);
    }

    process_utf8_chars(line, ctx)
}

fn process_utf8_chars<W: Write>(line: &str, ctx: &mut FoldContext<'_, W>) -> UResult<()> {
    let line_bytes = line.as_bytes();
    let mut iter = line.char_indices().peekable();

    while let Some((byte_idx, ch)) = iter.next() {
        // Include combining characters with the base character when we are
        // measuring by display columns. In character-counting mode every
        // scalar value must advance the counter to match `chars().count()`
        // semantics (see `fold_characters_reference` in the tests), so we do
        // not coalesce zero-width scalars there.
        if ctx.mode == WidthMode::Columns {
            while let Some(&(_, next_ch)) = iter.peek() {
                if UnicodeWidthChar::width(next_ch).unwrap_or(1) == 0 {
                    iter.next();
                } else {
                    break;
                }
            }
        }

        let next_idx = iter.peek().map_or(line_bytes.len(), |(idx, _)| *idx);

        if ch == '\n' {
            *ctx.last_space = None;
            emit_output(ctx)?;
            continue;
        }

        if *ctx.col_count >= ctx.width {
            emit_output(ctx)?;
        }

        if ch == '\r' {
            push_bytes(ctx, &line_bytes[byte_idx..next_idx])?;
            *ctx.col_count = 0;
            continue;
        }

        if ch == '\x08' {
            push_bytes(ctx, &line_bytes[byte_idx..next_idx])?;
            *ctx.col_count = ctx.col_count.saturating_sub(1);
            continue;
        }

        if ch == '\t' {
            loop {
                let next_stop = next_tab_stop(*ctx.col_count);
                if next_stop > ctx.width && !ctx.output.is_empty() {
                    emit_output(ctx)?;
                    continue;
                }
                *ctx.col_count = next_stop;
                break;
            }
            if ctx.break_mode == BreakMode::Hard {
                *ctx.last_space = None;
            } else {
                *ctx.last_space = Some(ctx.output.len());
            }
            push_bytes(ctx, &line_bytes[byte_idx..next_idx])?;
            continue;
        }

        let added = match ctx.mode {
            WidthMode::Columns => UnicodeWidthChar::width(ch).unwrap_or(0),
            WidthMode::Characters => 1,
        };

        if ctx.mode == WidthMode::Columns
            && added > 0
            && *ctx.col_count + added > ctx.width
            && !ctx.output.is_empty()
        {
            emit_output(ctx)?;
        }

        let cur_pos = ctx.output.len();
        match ctx.break_mode {
            BreakMode::Punctuation => {
                if ch.is_ascii() {
                    let b = ch as u8;
                    match b {
                        b'.' | b'!' | b'?' | b':' => {
                            *ctx.last_sentence_end = Some(cur_pos);
                        }
                        b',' | b';' => {
                            *ctx.last_clause_end = Some(cur_pos);
                        }
                        b if b.is_ascii_whitespace() && b != CR => {
                            *ctx.last_space = Some(cur_pos);
                        }
                        b if !b.is_ascii_alphanumeric() => {
                            *ctx.last_non_alnum = Some(cur_pos);
                        }
                        _ => {}
                    }
                } else if !ch.is_alphanumeric() {
                    *ctx.last_non_alnum = Some(cur_pos);
                }
            }
            BreakMode::Spaces => {
                if ch.is_ascii_whitespace() {
                    *ctx.last_space = Some(cur_pos);
                }
            }
            BreakMode::Hard => {}
        }

        push_bytes(ctx, &line_bytes[byte_idx..next_idx])?;
        *ctx.col_count = ctx.col_count.saturating_add(added);
    }

    Ok(())
}

fn process_non_utf8_line<W: Write>(line: &[u8], ctx: &mut FoldContext<'_, W>) -> UResult<()> {
    for &byte in line {
        if byte == NL {
            *ctx.last_space = None;
            *ctx.last_sentence_end = None;
            *ctx.last_clause_end = None;
            *ctx.last_non_alnum = None;
            emit_output(ctx)?;
            continue;
        }

        if *ctx.col_count >= ctx.width {
            emit_output(ctx)?;
        }

        match byte {
            CR => *ctx.col_count = 0,
            TAB => {
                let next_stop = next_tab_stop(*ctx.col_count);
                if next_stop > ctx.width && !ctx.output.is_empty() {
                    emit_output(ctx)?;
                }
                *ctx.col_count = next_stop;
                if ctx.break_mode != BreakMode::Hard {
                    *ctx.last_space = Some(ctx.output.len());
                }
                push_byte(ctx, byte)?;
                continue;
            }
            0x08 => *ctx.col_count = ctx.col_count.saturating_sub(1),
            _ => {
                let cur_pos = ctx.output.len();
                match ctx.break_mode {
                    BreakMode::Punctuation => match byte {
                        b'.' | b'!' | b'?' | b':' => {
                            *ctx.last_sentence_end = Some(cur_pos);
                        }
                        b',' | b';' => {
                            *ctx.last_clause_end = Some(cur_pos);
                        }
                        b if b.is_ascii_whitespace() && b != CR => {
                            *ctx.last_space = Some(cur_pos);
                        }
                        b if !b.is_ascii_alphanumeric() => {
                            *ctx.last_non_alnum = Some(cur_pos);
                        }
                        _ => {}
                    },
                    BreakMode::Spaces => {
                        if byte.is_ascii_whitespace() && byte != CR {
                            *ctx.last_space = Some(cur_pos);
                        }
                    }
                    BreakMode::Hard => {}
                }
                *ctx.col_count = ctx.col_count.saturating_add(1);
            }
        }

        push_byte(ctx, byte)?;
    }

    Ok(())
}

/// Process buffered bytes, emitting output for valid UTF-8 prefixes and
/// deferring incomplete sequences until more input arrives.
///
/// If the buffer contains invalid UTF-8, it is handled in non-UTF-8 mode and
/// the buffer is fully consumed.
fn process_pending_chunk<W: Write>(
    pending: &mut Vec<u8>,
    ctx: &mut FoldContext<'_, W>,
) -> UResult<()> {
    while !pending.is_empty() {
        match std::str::from_utf8(pending) {
            Ok(valid) => {
                process_utf8_line(valid, ctx)?;
                pending.clear();
                break;
            }
            Err(err) => {
                if err.error_len().is_some() {
                    let res = process_non_utf8_line(pending, ctx);
                    pending.clear();
                    res?;
                    break;
                }

                let valid_up_to = err.valid_up_to();
                if valid_up_to == 0 {
                    break;
                }

                let valid = std::str::from_utf8(&pending[..valid_up_to]).expect("valid prefix");
                process_utf8_line(valid, ctx)?;
                pending.drain(..valid_up_to);
            }
        }
    }

    Ok(())
}

/// Fold `file` to fit `width` (number of columns).
///
/// By default `fold` treats tab, backspace, and carriage return specially:
/// tab characters count as 8 columns, backspace decreases the
/// column count, and carriage return resets the column count to 0.
///
/// The `break_mode` controls where lines are broken: at whitespace (`Spaces`),
/// at tiered punctuation boundaries (`Punctuation`), or at the hard width limit.
#[allow(unused_assignments)]
#[allow(clippy::cognitive_complexity)]
fn fold_file<T: Read, W: Write>(
    mut file: BufReader<T>,
    break_mode: BreakMode,
    width: usize,
    mode: WidthMode,
    writer: &mut W,
) -> UResult<()> {
    let mut output = Vec::new();
    let mut col_count = 0;
    let mut last_space = None;
    let mut last_sentence_end = None;
    let mut last_clause_end = None;
    let mut last_non_alnum = None;
    let mut pending = Vec::with_capacity(8 * 1024);

    {
        let mut ctx = FoldContext {
            break_mode,
            width,
            mode,
            writer,
            output: &mut output,
            col_count: &mut col_count,
            last_space: &mut last_space,
            last_sentence_end: &mut last_sentence_end,
            last_clause_end: &mut last_clause_end,
            last_non_alnum: &mut last_non_alnum,
        };

        loop {
            let buffer = file
                .fill_buf()
                .map_err_context(|| translate!("fold-error-readline"))?;
            if buffer.is_empty() {
                break;
            }
            pending.extend_from_slice(buffer);
            let consumed = buffer.len();
            file.consume(consumed);

            process_pending_chunk(&mut pending, &mut ctx)?;
        }

        if !pending.is_empty() {
            match std::str::from_utf8(&pending) {
                Ok(s) => process_utf8_line(s, &mut ctx)?,
                Err(_) => process_non_utf8_line(&pending, &mut ctx)?,
            }
            pending.clear();
        }

        if !ctx.output.is_empty() {
            ctx.writer.write_all(ctx.output)?;
            ctx.output.clear();
        }
    }

    Ok(())
}
