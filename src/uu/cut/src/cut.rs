// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim foxjumping sourcefiles undelimited xacfoxjumping

use bstr::io::BufReadExt;
use clap::builder::{PossibleValue, ValueParser};
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, IsTerminal, Read, Write, stdin, stdout};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError, set_exit_code, strip_errno};
use uucore::i18n::charmap::{Encoding, locale_encoding, mb_char_len};
use uucore::line_ending::LineEnding;
use uucore::os_str_as_bytes;
use uucore::parser::shortcut_value_parser::ShortcutValueParser;

use self::searcher::Searcher;
use matcher::{ExactMatcher, Matcher, MbExactMatcher, WhitespaceMatcher};
use uucore::ranges::{Range, RangeError, RangeErrorKind};
use uucore::translate;
use uucore::{format_usage, show_error, show_if_err};

mod matcher;
mod searcher;

struct Options<'a> {
    out_delimiter: Option<&'a [u8]>,
    line_ending: LineEnding,
    field_opts: Option<FieldOptions<'a>>,
    /// `-n`: with `-b`, do not split multi-byte characters across the selection.
    suppress_split: bool,
}

enum Delimiter<'a> {
    Whitespace,
    Slice(&'a [u8]),
}

struct FieldOptions<'a> {
    delimiter: Delimiter<'a>,
    only_delimited: bool,
    /// `--whitespace-delimited=trimmed`: strip leading/trailing blanks per line.
    whitespace_trimmed: bool,
}

enum Mode<'a> {
    Bytes(Vec<Range>, Options<'a>),
    Characters(Vec<Range>, Options<'a>),
    Fields(Vec<Range>, Options<'a>),
}

impl Default for Delimiter<'_> {
    fn default() -> Self {
        Self::Slice(b"\t")
    }
}

impl<'a> From<&'a OsString> for Delimiter<'a> {
    fn from(s: &'a OsString) -> Self {
        Self::Slice(os_str_as_bytes(s).unwrap())
    }
}

/// GNU's range-list diagnostics, worded for field mode (`-f`/`-F`) or for
/// byte/character mode (`-b`/`-c`).
struct RangeErrors {
    numbered_from_1: &'static str,
    invalid_range: &'static str,
    invalid_value: &'static str,
    too_large: &'static str,
}

const FIELD_ERRORS: RangeErrors = RangeErrors {
    numbered_from_1: "cut-error-field-numbered-from-1",
    invalid_range: "cut-error-invalid-field-range",
    invalid_value: "cut-error-invalid-field-value",
    too_large: "cut-error-field-number-too-large",
};

const POSITION_ERRORS: RangeErrors = RangeErrors {
    numbered_from_1: "cut-error-position-numbered-from-1",
    invalid_range: "cut-error-invalid-position-range",
    invalid_value: "cut-error-invalid-position-value",
    too_large: "cut-error-position-too-large",
};

/// Fill in a message that names the offending text. Quoting it also keeps the
/// number away from Fluent's numeric formatting.
fn about(key: &str, text: &str) -> String {
    translate!(key, "value" => text.quote())
}

/// Split the leading run of ASCII digits off `s`, returning it along with the
/// rest of `s`. The run is empty when `s` does not start with a digit.
fn split_digits(s: &str) -> (&str, &str) {
    s.split_at(s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len()))
}

/// One item of a range list that does not parse: GNU's wording for it, plus
/// which label a caret should carry. The span is added by the caller, which is
/// the only place that knows where in the list the item sat.
struct ItemError {
    message: String,
    kind: RangeErrorKind,
}

/// Parse one endpoint of a range, `default` standing in for an omitted one.
/// GNU rejects `usize::MAX` itself, so the highest value taken is `MAX - 1`.
fn parse_endpoint(digits: &str, default: usize, errors: &RangeErrors) -> Result<usize, ItemError> {
    if digits.is_empty() {
        return Ok(default);
    }
    match digits.parse::<usize>() {
        Ok(n) if n != usize::MAX => Ok(n),
        _ => Err(ItemError {
            message: about(errors.too_large, digits),
            kind: RangeErrorKind::TooLarge,
        }),
    }
}

/// Parse a single `N`, `N-`, `-N` or `N-M` item into a range.
fn parse_range(item: &str, errors: &RangeErrors) -> Result<Range, ItemError> {
    let (low_digits, rest) = split_digits(item);
    // GNU reports everything it could not consume, not just the first byte.
    if low_digits.is_empty() && !rest.starts_with('-') {
        return Err(ItemError {
            message: about(errors.invalid_value, item),
            kind: RangeErrorKind::NotANumber,
        });
    }
    let Some(high_part) = rest.strip_prefix('-') else {
        // No dash at all: a lone number, or trailing junk after it.
        if !rest.is_empty() {
            return Err(ItemError {
                message: about(errors.invalid_value, rest),
                kind: RangeErrorKind::NotANumber,
            });
        }
        let n = parse_endpoint(low_digits, 0, errors)?;
        return Ok(Range { low: n, high: n });
    };

    let (high_digits, tail) = split_digits(high_part);
    if !tail.is_empty() {
        // A second dash makes the item a malformed range, anything else a bad
        // value.
        return Err(if tail.starts_with('-') {
            ItemError {
                message: translate!(errors.invalid_range),
                kind: RangeErrorKind::NoEndpoint,
            }
        } else {
            ItemError {
                message: about(errors.invalid_value, tail),
                kind: RangeErrorKind::NotANumber,
            }
        });
    }
    if low_digits.is_empty() && high_digits.is_empty() {
        return Err(ItemError {
            message: translate!("cut-error-invalid-range-no-endpoint", "range" => item),
            kind: RangeErrorKind::NoEndpoint,
        });
    }

    let low = parse_endpoint(low_digits, 1, errors)?;
    let high = parse_endpoint(high_digits, usize::MAX - 1, errors)?;
    if low > high {
        return Err(ItemError {
            message: translate!("cut-error-invalid-decreasing-range"),
            kind: RangeErrorKind::Inverted,
        });
    }
    Ok(Range { low, high })
}

/// Parse a range list, reporting GNU's exact diagnostics.
///
/// GNU distinguishes its messages by mode and by the kind of problem, which the
/// shared [`Range::from_list`] parser does not, so the list is parsed here and
/// the resulting ranges are merged directly.
fn parse_range_list(list: &str, is_field: bool) -> Result<Vec<Range>, RangeError> {
    let errors = if is_field {
        &FIELD_ERRORS
    } else {
        &POSITION_ERRORS
    };
    let mut ranges = Vec::new();
    // Where the current item starts inside `list`; the separators are one byte
    // each, so stepping past an item is its length plus one.
    let mut at = 0;

    for item in list.split([',', ' ']) {
        let span = at..at + item.len();
        at = span.end + 1;
        // A stray separator leaves an empty item, which GNU reads as position
        // zero -- the same complaint as an explicit `0`.
        let zero_bound = || RangeError {
            message: translate!(errors.numbered_from_1),
            span: span.clone(),
            kind: RangeErrorKind::ZeroBound,
        };
        if item.is_empty() {
            return Err(zero_bound());
        }
        let range = parse_range(item, errors).map_err(|e| RangeError {
            message: e.message,
            span: span.clone(),
            kind: e.kind,
        })?;
        if range.low == 0 {
            return Err(zero_bound());
        }
        ranges.push(range);
    }

    Ok(Range::merge(ranges))
}

fn list_to_ranges(list: &str, complement: bool, is_field: bool) -> Result<Vec<Range>, RangeError> {
    let ranges = parse_range_list(list, is_field)?;
    Ok(if complement {
        uucore::ranges::complement(&ranges)
    } else {
        ranges
    })
}

/// Write the parts of `line` selected by `ranges`, treating every byte as a
/// character.
///
/// Always inlined: it is the body of the per-line loop, and a call per line
/// costs more than the work it does on short lines.
#[inline(always)]
fn write_line_bytes<W: Write>(
    line: &[u8],
    out: &mut W,
    ranges: &[Range],
    out_delim: &[u8],
    explicit_delim: bool,
) -> std::io::Result<()> {
    let mut print_delim = false;
    for &Range { low, high } in ranges {
        if low > line.len() {
            break;
        }
        if print_delim {
            out.write_all(out_delim)?;
        } else if explicit_delim {
            print_delim = true;
        }
        // change `low` from 1-indexed value to 0-index value
        let low = low - 1;
        let high = high.min(line.len());
        out.write_all(&line[low..high])?;
    }
    Ok(())
}

fn cut_bytes<R: Read, W: Write>(
    reader: R,
    out: &mut W,
    ranges: &[Range],
    opts: &Options,
) -> UResult<()> {
    let newline_char = opts.line_ending.into();
    let mut buf_in = BufReader::new(reader);
    let out_delim = opts.out_delimiter.unwrap_or(b"\t");
    let explicit_delim = opts.out_delimiter.is_some();

    buf_in
        .for_byte_record(newline_char, |line| {
            write_line_bytes(line, out, ranges, out_delim, explicit_delim)?;
            out.write_all(&[newline_char])?;
            Ok(true)
        })
        .map_err(|e| USimpleError::new(1, strip_errno(&e)))
}

/// Offset of the first byte above `0x7F` in `bytes`, or `bytes.len()` if there
/// is none.
///
/// Whole words are scanned at a time while at least one is left, then the tail
/// byte by byte. Callers pass exactly the bytes they may consume, so the word
/// loop never has to discount bytes reaching past the end.
#[inline(always)]
fn ascii_run(bytes: &[u8]) -> usize {
    const HIGH_BITS: u64 = 0x8080_8080_8080_8080;

    let mut idx = 0;
    while let Some(chunk) = bytes[idx..].first_chunk::<8>() {
        let high = u64::from_le_bytes(*chunk) & HIGH_BITS;
        if high != 0 {
            return idx + high.trailing_zeros() as usize / 8;
        }
        idx += 8;
    }
    while idx < bytes.len() && bytes[idx] < 0x80 {
        idx += 1;
    }
    idx
}

/// Everything the character-mode line loop needs besides the line itself. It is
/// all fixed for the whole run, so it is built once and passed by reference:
/// handing these over one argument per line costs more than the work done on a
/// short line.
struct CharCut<'a> {
    ranges: &'a [Range],
    out_delim: &'a [u8],
    explicit_delim: bool,
    /// `-c`: a position is the index of the character. Otherwise (`-b -n`) it
    /// is the offset of the character's last byte, which is what GNU selects on.
    by_char: bool,
    encoding: Encoding,
}

impl CharCut<'_> {
    /// Walk `line` from byte offset `idx` (at position `pos`) as long as the
    /// position of the last consumed character stays within `limit`, and return
    /// the byte offset and position reached.
    #[inline(always)]
    fn advance(&self, line: &[u8], mut idx: usize, mut pos: usize, limit: usize) -> (usize, usize) {
        while pos < limit && idx < line.len() {
            // ASCII bytes are single-byte characters in every encoding handled
            // here, so offset and position move together and a whole run of
            // them can be taken at once, without going through the decoder.
            let room = (limit - pos).min(line.len() - idx);
            let run = ascii_run(&line[idx..idx + room]);
            idx += run;
            pos += run;
            if run == room {
                break;
            }
            let len = self.encoding.char_len(&line[idx..]); // in `1..=line.len() - idx`
            let step = if self.by_char { 1 } else { len };
            if pos + step > limit {
                break;
            }
            idx += len;
            pos += step;
        }
        (idx, pos)
    }

    /// Write the parts of `line` selected by the ranges, keeping multi-byte
    /// characters whole. A character belongs to a range when its position is in
    /// `low..=high`.
    ///
    /// Always inlined, like its byte counterpart: it is the body of the
    /// per-line loop, and the call costs more than the work done on a short
    /// line.
    #[inline(always)]
    fn write_line<W: Write>(&self, line: &[u8], out: &mut W) -> std::io::Result<()> {
        let mut print_delim = false;
        // Byte offset of the next character to look at, and the position
        // already consumed. The ranges are sorted and disjoint, so one pass
        // over the line is enough and each range maps to a contiguous slice.
        let (mut idx, mut pos) = (0, 0);
        // End of the previous range, and end of the last range before the run
        // of adjacent ranges the current one belongs to (1-based, `0` for
        // none), plus whether a delimiter is owed to the next character.
        let (mut prev_high, mut before_run_high, mut delim_pending) = (0, 0, false);
        for &Range { low, high } in self.ranges {
            // A character position is never below its own byte offset, so a
            // range starting past the last byte selects nothing, and so do the
            // ones after it.
            if low > line.len() {
                break;
            }
            // Skip the characters located before the range.
            (idx, pos) = self.advance(line, idx, pos, low - 1);
            if idx == line.len() {
                break;
            }
            // Adjacent ranges select a contiguous stretch of the line, so they
            // form one run as far as characters are concerned.
            if prev_high + 1 != low {
                before_run_high = prev_high;
            }
            // A range boundary only separates characters when it falls between
            // two of them; the delimiter is owed to whichever character prints
            // next, possibly from a later range. Positions are byte offsets
            // here, so all this only concerns `-b -n`: a character position
            // can never land inside a character.
            if self.by_char || idx >= prev_high {
                delim_pending = true;
            }
            // The selected bytes of a character must reach its end without a
            // hole, so one an earlier run reached into is dropped even though
            // its last byte falls here, taking the owed delimiter with it.
            if !self.by_char && idx < before_run_high {
                delim_pending = false;
                let len = self.encoding.char_len(&line[idx..]);
                idx += len;
                pos += len;
                if idx >= line.len() {
                    break;
                }
            }
            prev_high = high;
            let start = idx;
            if high >= line.len() {
                // The range reaches past the end of the line, so it covers
                // every character left and none of them needs to be decoded.
                // The ranges after it start further away still.
                if print_delim && delim_pending {
                    out.write_all(self.out_delim)?;
                }
                return out.write_all(&line[start..]);
            }
            (idx, pos) = self.advance(line, idx, pos, high);
            // A range covering only part of a multi-byte character selects
            // nothing, and leaves the delimiter it owes to the next one.
            if idx == start {
                continue;
            }
            if print_delim {
                if delim_pending {
                    out.write_all(self.out_delim)?;
                }
            } else if self.explicit_delim {
                print_delim = true;
            }
            delim_pending = false;
            out.write_all(&line[start..idx])?;
        }
        Ok(())
    }
}

/// Cut `-c` (whole characters) or `-b -n` (bytes, keeping whole characters).
///
/// In a single-byte locale, or for `-b` without `-n`, this falls back to the
/// plain byte path. Otherwise each character is emitted whole when its 1-based
/// position falls in a range: the character index for `-c`, or the offset of
/// its last byte for `-b -n` (matching GNU).
fn cut_chars<R: Read, W: Write>(
    reader: R,
    out: &mut W,
    ranges: &[Range],
    opts: &Options,
    by_char: bool,
) -> UResult<()> {
    let encoding = locale_encoding();
    if encoding == Encoding::SingleByte || !(by_char || opts.suppress_split) {
        return cut_bytes(reader, out, ranges, opts);
    }

    let newline_char = opts.line_ending.into();
    let mut buf_in = BufReader::new(reader);
    let cut = CharCut {
        ranges,
        out_delim: opts.out_delimiter.unwrap_or(b"\t"),
        explicit_delim: opts.out_delimiter.is_some(),
        by_char,
        encoding,
    };

    buf_in
        .for_byte_record(newline_char, |line| {
            cut.write_line(line, out)?;
            out.write_all(&[newline_char])?;
            Ok(true)
        })
        .map_err(|e| USimpleError::new(1, strip_errno(&e)))
}

/// Write the fields of `line` selected by `ranges`, separated by `out_delim`.
///
/// `line` may or may not end with `newline_char`; either way the output is
/// terminated, unless the line has no delimiter and `only_delimited` drops it.
fn write_fields_line<W: Write, M: Matcher>(
    line: &[u8],
    out: &mut W,
    matcher: &M,
    ranges: &[Range],
    only_delimited: bool,
    newline_char: u8,
    out_delim: &[u8],
) -> std::io::Result<()> {
    let mut fields_pos = 1;
    let mut low_idx = 0;
    let mut delim_search = Searcher::new(matcher, line).peekable();
    let mut print_delim = false;

    if delim_search.peek().is_none() {
        if !only_delimited {
            // Always write the entire line, even if it doesn't end with `newline_char`
            out.write_all(line)?;
            if line.is_empty() || line[line.len() - 1] != newline_char {
                out.write_all(&[newline_char])?;
            }
        }

        return Ok(());
    }

    for &Range { low, high } in ranges {
        if low - fields_pos > 0 {
            // current field is not in the range, so jump to the field corresponding to the
            // beginning of the range if any
            low_idx = match delim_search.nth(low - fields_pos - 1) {
                Some((_, last)) => last,
                None => break,
            };
        }

        // at this point, current field is the first in the range
        for _ in 0..=high - low {
            // skip printing delimiter if this is the first matching field for this line
            if print_delim {
                out.write_all(out_delim)?;
            } else {
                print_delim = true;
            }

            if let Some((first, last)) = delim_search.next() {
                // print the current field up to the next field delim
                let segment = &line[low_idx..first];

                out.write_all(segment)?;

                low_idx = last;
                fields_pos = high + 1;
            } else {
                // this is the last field in the line, so print the rest
                let segment = &line[low_idx..];

                out.write_all(segment)?;

                if line[line.len() - 1] == newline_char {
                    return Ok(());
                }
                break;
            }
        }
    }

    out.write_all(&[newline_char])
}

/// Output delimiter is explicitly specified
fn cut_fields_explicit_out_delim<R: Read, W: Write, M: Matcher>(
    reader: R,
    out: &mut W,
    matcher: &M,
    ranges: &[Range],
    only_delimited: bool,
    newline_char: u8,
    out_delim: &[u8],
) -> UResult<()> {
    let mut buf_in = BufReader::new(reader);

    buf_in
        .for_byte_record_with_terminator(newline_char, |line| {
            write_fields_line(
                line,
                out,
                matcher,
                ranges,
                only_delimited,
                newline_char,
                out_delim,
            )?;
            Ok(true)
        })
        .map_err(|e| USimpleError::new(1, e.to_string()))
}

/// Output delimiter is the same as input delimiter
fn cut_fields_implicit_out_delim<R: Read, W: Write, M: Matcher>(
    reader: R,
    out: &mut W,
    matcher: &M,
    ranges: &[Range],
    only_delimited: bool,
    newline_char: u8,
) -> UResult<()> {
    let mut buf_in = BufReader::new(reader);

    buf_in
        .for_byte_record_with_terminator(newline_char, |line| {
            let mut fields_pos = 1;
            let mut low_idx = 0;
            let mut delim_search = Searcher::new(matcher, line).peekable();
            let mut print_delim = false;

            if delim_search.peek().is_none() {
                if !only_delimited {
                    // Always write the entire line, even if it doesn't end with `newline_char`
                    out.write_all(line)?;
                    if line.is_empty() || line[line.len() - 1] != newline_char {
                        out.write_all(&[newline_char])?;
                    }
                }

                return Ok(true);
            }

            for &Range { low, high } in ranges {
                if low - fields_pos > 0 {
                    if let Some((first, last)) = delim_search.nth(low - fields_pos - 1) {
                        low_idx = if print_delim { first } else { last }
                    } else {
                        break;
                    }
                }

                if let Some((first, _)) = delim_search.nth(high - low) {
                    let segment = &line[low_idx..first];

                    out.write_all(segment)?;

                    print_delim = true;
                    low_idx = first;
                    fields_pos = high + 1;
                } else {
                    let segment = &line[low_idx..line.len()];

                    out.write_all(segment)?;

                    if line[line.len() - 1] == newline_char {
                        return Ok(true);
                    }
                    break;
                }
            }
            out.write_all(&[newline_char])?;
            Ok(true)
        })
        .map_err(|e| USimpleError::new(1, e.to_string()))
}

/// Streams and filters fields where the record terminator and
/// field delimiter are the same character (specified by `newline_char`)
fn cut_fields_newline_char_delim<R: Read, W: Write>(
    reader: R,
    out: &mut W,
    ranges: &[Range],
    newline_char: u8,
    out_delim: &[u8],
    only_delimited: bool,
) -> UResult<()> {
    let mut reader = BufReader::new(reader);
    let mut line = Vec::new();

    // We start at 1 because 'cut' field indexing is 1-based
    let mut current_field_idx = 1;
    let mut first_field_printed = false;
    let mut has_data = false;
    let mut suppressed = false;

    let mut range_idx = 0;

    loop {
        line.clear();

        let is_selected = range_idx < ranges.len() && current_field_idx >= ranges[range_idx].low;
        let needs_data = is_selected || current_field_idx == 1;

        let mut has_processed_data = false;

        if needs_data {
            // Standard read: copies bytes into `line`
            loop {
                let buf = reader.fill_buf()?;
                if buf.is_empty() {
                    break;
                }

                has_processed_data = true;

                if let Some(pos) = memchr::memchr(newline_char, buf) {
                    let amt = pos + 1;
                    line.extend_from_slice(&buf[..amt]);
                    reader.consume(amt);

                    break;
                }
                let len = buf.len();
                line.extend_from_slice(buf);
                reader.consume(len);
            }
        } else {
            // Zero-allocation skip: scans the buffer and advances the cursor without copying
            loop {
                let buf = reader.fill_buf()?;
                if buf.is_empty() {
                    break; // EOF
                }

                has_processed_data = true;

                if let Some(pos) = memchr::memchr(newline_char, buf) {
                    let bytes_to_consume = pos + 1;
                    reader.consume(bytes_to_consume);
                    break;
                }

                let len = buf.len();
                reader.consume(len);
            }
        }

        if !has_processed_data {
            break;
        }
        has_data = true;

        // To comply with -s when the stream consists of only a single field.
        if current_field_idx == 1 {
            let is_eof_next = reader.fill_buf()?.is_empty();

            if is_eof_next && line.last() != Some(&newline_char) {
                if only_delimited {
                    suppressed = true;
                } else {
                    // GNU cut prints the whole line if no delimiter is found.
                    out.write_all(&line)?;
                }
                break;
            }
        }

        if range_idx < ranges.len() && current_field_idx > ranges[range_idx].high {
            range_idx += 1;

            // EARLY EXIT: If we've exhausted all ranges, stop reading the stream entirely.
            if range_idx == ranges.len() {
                break;
            }
        }

        // Check if the current field falls inside the current active range
        let is_selected = range_idx < ranges.len() && current_field_idx >= ranges[range_idx].low;

        if is_selected {
            if first_field_printed {
                out.write_all(out_delim)?;
            }

            let has_newline = line.last() == Some(&newline_char);
            let content = if has_newline {
                &line[..line.len() - 1]
            } else {
                &line[..]
            };

            out.write_all(content)?;
            first_field_printed = true;
        }

        current_field_idx += 1;
    }

    // With -s, a record that produced no selected field must not emit a line.
    if has_data && !suppressed && (first_field_printed || !only_delimited) {
        out.write_all(&[newline_char])?;
    }

    Ok(())
}

/// `--whitespace-delimited=trimmed`: strip leading/trailing blanks from each
/// line, then split on runs of blanks. With no internal blank the line is
/// undelimited (suppressed under `-s`).
fn cut_fields_whitespace_trimmed<R: Read, W: Write>(
    reader: R,
    out: &mut W,
    ranges: &[Range],
    only_delimited: bool,
    newline_char: u8,
    out_delim: &[u8],
) -> UResult<()> {
    let is_blank = |&b: &u8| b == b' ' || b == b'\t';
    let matcher = WhitespaceMatcher {};
    let mut buf_in = BufReader::new(reader);

    // Trimming is the only difference from the plain whitespace path: once the
    // outer blanks are gone, the remaining blank runs are ordinary delimiters.
    buf_in
        .for_byte_record(newline_char, |line| {
            let start = line.iter().position(|b| !is_blank(b)).unwrap_or(line.len());
            let end = line
                .iter()
                .rposition(|b| !is_blank(b))
                .map_or(start, |p| p + 1);

            write_fields_line(
                &line[start..end],
                out,
                &matcher,
                ranges,
                only_delimited,
                newline_char,
                out_delim,
            )?;
            Ok(true)
        })
        .map_err(|e| USimpleError::new(1, e.to_string()))
}

/// Run field cutting with the given matcher, choosing the explicit- or
/// implicit-output-delimiter routine.
fn cut_fields_with_matcher<R: Read, W: Write, M: Matcher>(
    reader: R,
    out: &mut W,
    matcher: &M,
    ranges: &[Range],
    only_delimited: bool,
    newline_char: u8,
    out_delimiter: Option<&[u8]>,
) -> UResult<()> {
    match out_delimiter {
        Some(out_delim) => cut_fields_explicit_out_delim(
            reader,
            out,
            matcher,
            ranges,
            only_delimited,
            newline_char,
            out_delim,
        ),
        None => cut_fields_implicit_out_delim(
            reader,
            out,
            matcher,
            ranges,
            only_delimited,
            newline_char,
        ),
    }
}

fn cut_fields<R: Read, W: Write>(
    reader: R,
    out: &mut W,
    ranges: &[Range],
    opts: &Options,
) -> UResult<()> {
    let newline_char = opts.line_ending.into();
    let field_opts = opts.field_opts.as_ref().unwrap(); // it is safe to unwrap() here - field_opts will always be Some() for cut_fields() call
    match field_opts.delimiter {
        Delimiter::Slice(delim) if delim == [newline_char] => {
            let out_delim = opts.out_delimiter.unwrap_or(delim);
            cut_fields_newline_char_delim(
                reader,
                out,
                ranges,
                newline_char,
                out_delim,
                field_opts.only_delimited,
            )
        }
        // An ASCII single-byte delimiter can never occur inside a multi-byte
        // character, so the fast byte-wise matcher is correct in every locale.
        // Otherwise match the delimiter as a whole character.
        Delimiter::Slice(delim) if delim.len() == 1 && delim[0] <= 0x7F => cut_fields_with_matcher(
            reader,
            out,
            &ExactMatcher::new(delim),
            ranges,
            field_opts.only_delimited,
            newline_char,
            opts.out_delimiter,
        ),
        Delimiter::Slice(delim) => cut_fields_with_matcher(
            reader,
            out,
            &MbExactMatcher::new(delim),
            ranges,
            field_opts.only_delimited,
            newline_char,
            opts.out_delimiter,
        ),
        Delimiter::Whitespace => {
            let out_delim = opts.out_delimiter.unwrap_or(b"\t");
            if field_opts.whitespace_trimmed {
                cut_fields_whitespace_trimmed(
                    reader,
                    out,
                    ranges,
                    field_opts.only_delimited,
                    newline_char,
                    out_delim,
                )
            } else {
                let matcher = WhitespaceMatcher {};
                cut_fields_explicit_out_delim(
                    reader,
                    out,
                    &matcher,
                    ranges,
                    field_opts.only_delimited,
                    newline_char,
                    out_delim,
                )
            }
        }
    }
}

fn cut_files<'a, I>(filenames: I, mode: &Mode)
where
    I: IntoIterator<Item = &'a OsString>,
{
    let mut stdin_read = false;
    let mut out: Box<dyn Write> = if stdout().is_terminal() {
        Box::new(stdout())
    } else {
        Box::new(BufWriter::new(stdout())) as Box<dyn Write>
    };

    for filename in filenames {
        if filename == "-" {
            if stdin_read {
                continue;
            }

            show_if_err!(match mode {
                Mode::Bytes(ranges, opts) => cut_chars(stdin(), &mut out, ranges, opts, false),
                Mode::Characters(ranges, opts) => cut_chars(stdin(), &mut out, ranges, opts, true),
                Mode::Fields(ranges, opts) => cut_fields(stdin(), &mut out, ranges, opts),
            });

            stdin_read = true;
        } else {
            let path = Path::new(filename);

            if path.is_dir() {
                show_error!(
                    "{}: {}",
                    filename.maybe_quote(),
                    translate!("cut-error-is-directory")
                );
                set_exit_code(1);
                continue;
            }

            show_if_err!(
                File::open(path)
                    .map_err_context(|| filename.maybe_quote().to_string())
                    .and_then(|file| {
                        match &mode {
                            Mode::Bytes(ranges, opts) => {
                                cut_chars(file, &mut out, ranges, opts, false)
                            }
                            Mode::Characters(ranges, opts) => {
                                cut_chars(file, &mut out, ranges, opts, true)
                            }
                            Mode::Fields(ranges, opts) => cut_fields(file, &mut out, ranges, opts),
                        }
                    })
            );
        }
    }

    show_if_err!(
        out.flush()
            .map_err_context(|| translate!("cut-error-write-error"))
    );
}

/// Get delimiter and output delimiter from `-d`/`--delimiter` and `--output-delimiter` options respectively
/// Allow either delimiter to have a value that is neither UTF-8 nor ASCII to align with GNU behavior
fn get_delimiters(matches: &ArgMatches) -> UResult<(Delimiter<'_>, Option<&[u8]>)> {
    let whitespace_delimited = matches.contains_id(options::WHITESPACE_DELIMITED);
    let delim_opt = matches.get_one::<OsString>(options::DELIMITER);
    let delim = match delim_opt {
        Some(_) if whitespace_delimited => {
            return Err(UUsageError::new(
                1,
                translate!("cut-error-delimiter-and-whitespace-conflict"),
            ));
        }
        Some(os_string) => {
            if os_string.is_empty() {
                Delimiter::Slice(b"\0")
            } else {
                // The delimiter must be a single character in the *current locale's*
                // encoding: a multi-byte UTF-8 character in a UTF-8 locale, a 2-byte
                // GB18030 character, or any single byte like `b"\xAD"`. In a C/POSIX
                // (single-byte) locale even a valid UTF-8 multibyte sequence counts
                // as several characters, matching GNU.
                let bytes = os_str_as_bytes(os_string)?;
                let is_single_char = mb_char_len(bytes) == bytes.len();
                if !is_single_char {
                    return Err(UUsageError::new(
                        1,
                        translate!("cut-error-delimiter-must-be-single-character"),
                    ));
                }
                Delimiter::from(os_string)
            }
        }
        None => {
            if whitespace_delimited {
                Delimiter::Whitespace
            } else {
                Delimiter::default()
            }
        }
    };
    let out_delim = matches
        .get_one::<OsString>(options::OUTPUT_DELIMITER)
        .map(|os_string| {
            if os_string.is_empty() {
                b"\0"
            } else {
                os_str_as_bytes(os_string).unwrap()
            }
        });
    Ok((delim, out_delim))
}

mod options {
    pub const BYTES: &str = "bytes";
    pub const CHARACTERS: &str = "characters";
    pub const DELIMITER: &str = "delimiter";
    pub const FIELDS: &str = "fields";
    // -F: field mode that merges whitespace (or -d) runs, default output is a space.
    pub const FIELDS_MERGED: &str = "fields-merged";
    pub const ZERO_TERMINATED: &str = "zero-terminated";
    pub const ONLY_DELIMITED: &str = "only-delimited";
    pub const OUTPUT_DELIMITER: &str = "output-delimiter";
    pub const WHITESPACE_DELIMITED: &str = "whitespace-delimited";
    pub const COMPLEMENT: &str = "complement";
    pub const FILE: &str = "file";
    pub const NO_PARTIAL: &str = "no-partial";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args: Vec<OsString> = args.collect();
    // Kept for the caret in list diagnostics, which needs the arguments as
    // they were typed, before the rewrite below edits one of them.
    let diag_args = uucore::diagnostics::capture(&args);

    // GNU `cut` supports `-d=` to set the delimiter to `=`.
    // Clap parsing is limited in this situation, see:
    // https://github.com/uutils/coreutils/issues/2424#issuecomment-863825242
    let args = args.into_iter().map(|x| {
        if x == "-d=" {
            "--delimiter==".into()
        } else {
            x
        }
    });

    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let complement = matches.get_flag(options::COMPLEMENT);
    let only_delimited = matches.get_flag(options::ONLY_DELIMITED);

    let (delimiter, out_delimiter) = get_delimiters(&matches)?;
    let line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO_TERMINATED));
    let suppress_split = matches.get_flag(options::NO_PARTIAL);
    // `--whitespace-delimited[=trimmed]` (`-w`): the optional value selects trimming.
    let whitespace_trimmed = matches
        .get_one::<String>(options::WHITESPACE_DELIMITED)
        .is_some();

    let mode_arg = get_mode_arg(&matches)?;
    let list = matches
        .get_one::<String>(mode_arg)
        .expect("should be ensured by get_mode_arg");
    let is_field = matches!(mode_arg, options::FIELDS | options::FIELDS_MERGED);
    let ranges = list_to_ranges(list, complement, is_field).map_err(|e| {
        // The list is the value of the option that selected the mode, so the
        // caret can be put under the one range that is at fault.
        let (short, long) = mode_arg_names(mode_arg);
        uucore::diagnostics::error_after_report(
            diag_args.as_deref(),
            UUsageError::new(1, e.message.clone()),
            |args, _| {
                e.render_option_value(
                    args,
                    list,
                    Some(short),
                    long,
                    &translate!("cut-diag-label-zero-bound"),
                    &translate!("cut-diag-help-list-syntax"),
                )
            },
        )
    })?;

    let mode = match mode_arg {
        options::BYTES => Mode::Bytes(
            ranges,
            Options {
                out_delimiter,
                line_ending,
                field_opts: None,
                suppress_split,
            },
        ),
        options::CHARACTERS => Mode::Characters(
            ranges,
            Options {
                out_delimiter,
                line_ending,
                field_opts: None,
                suppress_split,
            },
        ),
        options::FIELDS => Mode::Fields(
            ranges,
            Options {
                out_delimiter,
                line_ending,
                field_opts: Some(FieldOptions {
                    delimiter,
                    only_delimited,
                    whitespace_trimmed,
                }),
                suppress_split,
            },
        ),
        // `-F`: like `-f`, but consecutive delimiters merge, the delimiter
        // defaults to whitespace, and the output delimiter defaults to a space.
        options::FIELDS_MERGED => Mode::Fields(
            ranges,
            Options {
                out_delimiter: out_delimiter.or(Some(b" ")),
                line_ending,
                field_opts: Some(FieldOptions {
                    delimiter: if matches.contains_id(options::DELIMITER) {
                        delimiter
                    } else {
                        Delimiter::Whitespace
                    },
                    only_delimited,
                    whitespace_trimmed,
                }),
                suppress_split,
            },
        ),
        _ => unreachable!(),
    };

    #[allow(clippy::unwrap_used, reason = "clap provides '-' by default")]
    let files = matches.get_many::<OsString>(options::FILE).unwrap();

    cut_files(files, &mode);

    Ok(())
}

// Exactly one of the cutting mode arguments `-b`, `-c`, `-f` or `-F` must be
// given.
//
// Returns `options::BYTES`, `options::CHARACTERS`, `options::FIELDS`, or
// `options::FIELDS_MERGED`.
fn get_mode_arg(matches: &ArgMatches) -> UResult<&str> {
    let mode_args_and_counts: Vec<_> = [
        options::BYTES,
        options::CHARACTERS,
        options::FIELDS,
        options::FIELDS_MERGED,
    ]
    .into_iter()
    .filter_map(|arg| {
        let count = matches.indices_of(arg)?.count();
        (count > 0).then_some((arg, count))
    })
    .collect();

    let mode_arg = match mode_args_and_counts.as_slice() {
        [(arg, 1)] => *arg,
        [] => {
            return Err(UUsageError::new(
                1,
                translate!("cut-error-missing-mode-arg"),
            ));
        }
        _ => {
            return Err(UUsageError::new(
                1,
                translate!("cut-error-multiple-mode-args"),
            ));
        }
    };

    if matches!(mode_arg, options::BYTES | options::CHARACTERS) {
        let checks = [
            (
                matches.contains_id(options::DELIMITER),
                "cut-error-delimiter-only-with-fields",
            ),
            (
                // GNU words `-w` as a delimiter too, so it shares the message.
                matches.contains_id(options::WHITESPACE_DELIMITED),
                "cut-error-delimiter-only-with-fields",
            ),
            (
                matches.get_flag(options::ONLY_DELIMITED),
                "cut-error-only-delimited-only-with-fields",
            ),
        ];

        for (is_triggered, msg_key) in checks {
            if is_triggered {
                return Err(UUsageError::new(1, translate!(msg_key)));
            }
        }
    }

    Ok(mode_arg)
}

/// The names of a mode option, since the caret has to recognize the value
/// however it was written: `-f2-`, `-f 2-` or `--fields=2-`.
fn mode_arg_names(mode_arg: &str) -> (char, Option<&str>) {
    match mode_arg {
        options::BYTES => ('b', Some(options::BYTES)),
        options::CHARACTERS => ('c', Some(options::CHARACTERS)),
        options::FIELDS => ('f', Some(options::FIELDS)),
        options::FIELDS_MERGED => ('F', None),
        _ => unreachable!("unknown cut mode option"),
    }
}

pub fn uu_app() -> Command {
    Command::new("cut")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(format_usage(&translate!("cut-usage")))
        .about(translate!("cut-about"))
        .after_help(translate!("cut-after-help"))
        .infer_long_args(true)
        // While `args_override_self(true)` for some arguments, such as `-d`
        // and `--output-delimiter`, is consistent to the behavior of GNU cut,
        // arguments related to cutting mode, i.e. `-b`, `-c`, `-f`, should
        // cause an error when there is more than one of them, as described in
        // the manual of GNU cut: "Use one, and only one of -b, -c or -f".
        // `ArgAction::Append` is used on `-b`, `-c`, `-f` arguments, so that
        // the occurrences of those could be counted and be handled accordingly.
        .args_override_self(true)
        .arg(
            Arg::new(options::BYTES)
                .short('b')
                .long(options::BYTES)
                .help(translate!("cut-help-bytes"))
                .allow_hyphen_values(true)
                .value_name("LIST")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(options::CHARACTERS)
                .short('c')
                .long(options::CHARACTERS)
                .help(translate!("cut-help-characters"))
                .allow_hyphen_values(true)
                .value_name("LIST")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(options::DELIMITER)
                .short('d')
                .long(options::DELIMITER)
                .value_parser(ValueParser::os_string())
                .help(translate!("cut-help-delimiter"))
                .value_name("DELIM"),
        )
        .arg(
            Arg::new(options::WHITESPACE_DELIMITED)
                .short('w')
                .long(options::WHITESPACE_DELIMITED)
                .help(translate!("cut-help-whitespace-delimited"))
                .value_name("trimmed")
                .value_parser(ShortcutValueParser::new([PossibleValue::new("trimmed")]))
                .num_args(0..=1)
                .require_equals(true)
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new(options::FIELDS)
                .short('f')
                .long(options::FIELDS)
                .help(translate!("cut-help-fields"))
                .allow_hyphen_values(true)
                .value_name("LIST")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(options::FIELDS_MERGED)
                .short('F')
                .help(translate!("cut-help-fields-merged"))
                .allow_hyphen_values(true)
                .value_name("LIST")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(options::COMPLEMENT)
                .long(options::COMPLEMENT)
                .help(translate!("cut-help-complement"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ONLY_DELIMITED)
                .short('s')
                .long(options::ONLY_DELIMITED)
                .help(translate!("cut-help-only-delimited"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ZERO_TERMINATED)
                .short('z')
                .long(options::ZERO_TERMINATED)
                .help(translate!("cut-help-zero-terminated"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OUTPUT_DELIMITER)
                .short('O')
                .long(options::OUTPUT_DELIMITER)
                .value_parser(ValueParser::os_string())
                .help(translate!("cut-help-output-delimiter"))
                .value_name("NEW_DELIM"),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath)
                .default_value("-")
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::NO_PARTIAL)
                .short('n')
                .long(options::NO_PARTIAL)
                .help(translate!("cut-help-no-partial"))
                .action(ArgAction::SetTrue),
        )
}

#[cfg(test)]
mod tests {
    use super::{CharCut, Encoding};

    fn utf8_cut(by_char: bool) -> CharCut<'static> {
        CharCut {
            ranges: &[],
            out_delim: b"\t",
            explicit_delim: false,
            by_char,
            encoding: Encoding::Utf8,
        }
    }

    // "quick" + 2-byte char + "brown" + 3-byte char + "foxjumping"
    const LINE: &[u8] = b"quick\xc3\xa9brown\xe2\x82\xacfoxjumping";

    #[test]
    fn advance_counts_characters_for_c() {
        let cut = utf8_cut(true);
        // Inside the leading ASCII run, and stopping right on its last byte.
        assert_eq!(cut.advance(LINE, 0, 0, 3), (3, 3));
        assert_eq!(cut.advance(LINE, 0, 0, 5), (5, 5));
        // Taking the 2-byte character moves two bytes but one position.
        assert_eq!(cut.advance(LINE, 0, 0, 6), (7, 6));
        // Through "brown": 12 bytes for 11 characters, the 2-byte one included.
        assert_eq!(cut.advance(LINE, 0, 0, 11), (12, 11));
        // A limit past the end stops at the end: 25 bytes, 22 characters.
        assert_eq!(cut.advance(LINE, 0, 0, 99), (LINE.len(), 22));
        // Resuming mid-line, and a limit that is already reached.
        assert_eq!(cut.advance(LINE, 7, 6, 9), (10, 9));
        assert_eq!(cut.advance(LINE, 7, 6, 6), (7, 6));
    }

    #[test]
    fn advance_counts_bytes_for_b_n() {
        let cut = utf8_cut(false);
        // Positions are byte offsets here, so the ASCII head is unchanged.
        assert_eq!(cut.advance(LINE, 0, 0, 5), (5, 5));
        // The 2-byte character only fits once the limit covers both bytes.
        assert_eq!(cut.advance(LINE, 0, 0, 6), (5, 5));
        assert_eq!(cut.advance(LINE, 0, 0, 7), (7, 7));
        // Likewise the 3-byte one: 12 and 13 are short, 14 takes it.
        assert_eq!(cut.advance(LINE, 0, 0, 13), (12, 12));
        assert_eq!(cut.advance(LINE, 0, 0, 15), (15, 15));
    }

    #[test]
    fn advance_crosses_word_boundaries() {
        let cut = utf8_cut(true);
        // "foxjumping" is long enough for the word-at-a-time path, both
        // landing exactly on a word boundary and past one.
        let tail = &LINE[15..];
        assert_eq!(cut.advance(tail, 0, 0, 8), (8, 8));
        assert_eq!(cut.advance(tail, 0, 0, 10), (10, 10));
        // A high byte found by the word scan still yields to the decoder.
        let wide = b"abcdefghij\xc3\xa9kl";
        assert_eq!(cut.advance(wide, 0, 0, 11), (12, 11));
        // An empty line has nothing to walk.
        assert_eq!(cut.advance(b"", 0, 0, 4), (0, 0));
    }
}
