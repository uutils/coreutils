// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) sbytes slen dlen memmem memmap Mmap mmap SIGBUS

mod error;

use clap::{Arg, ArgAction, Command};
use memchr::memmem;
use memmap2::Mmap;
use std::ffi::{OsStr, OsString};
use std::io::{BufWriter, Read, Write, stdin, stdout};
use std::{fs::File, io::copy, path::Path};
#[cfg(unix)]
use uucore::error::UError;
use uucore::error::UResult;
#[cfg(unix)]
use uucore::error::set_exit_code;
use uucore::{format_usage, show};

use crate::error::TacError;

use uucore::translate;

mod options {
    pub static BEFORE: &str = "before";
    pub static REGEX: &str = "regex";
    pub static SEPARATOR: &str = "separator";
    pub static FILE: &str = "file";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let before = matches.get_flag(options::BEFORE);
    let regex = matches.get_flag(options::REGEX);
    let raw_separator = matches
        .get_one::<OsString>(options::SEPARATOR)
        .map_or(OsStr::new("\n"), |s| s.as_os_str());

    let separator = if raw_separator.is_empty() {
        OsStr::new("\0")
    } else {
        raw_separator
    };

    let files: Vec<OsString> = match matches.get_many::<OsString>(options::FILE) {
        Some(v) => v.cloned().collect(),
        None => vec![OsString::from("-")],
    };

    tac(&files, before, regex, separator)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(format_usage(&translate!("tac-usage")))
        .about(translate!("tac-about"))
        .infer_long_args(true)
        .arg(
            Arg::new(options::BEFORE)
                .short('b')
                .long(options::BEFORE)
                .help(translate!("tac-help-before"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REGEX)
                .short('r')
                .long(options::REGEX)
                .help(translate!("tac-help-regex"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SEPARATOR)
                .short('s')
                .long(options::SEPARATOR)
                .help(translate!("tac-help-separator"))
                .value_parser(clap::value_parser!(OsString))
                .value_name("STRING"),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_parser(clap::value_parser!(OsString))
                .value_hint(clap::ValueHint::FilePath),
        )
}

/// Print lines of a buffer in reverse, with line separator given as a regex.
///
/// `data` contains the bytes of the file.
///
/// `pattern` is the regular expression given as a
/// [`regex::bytes::Regex`] (not a [`regex::Regex`], since the input is
/// given as a slice of bytes). If `before` is `true`, then each match
/// of this pattern in `data` is interpreted as the start of a line. If
/// `before` is `false`, then each match of this pattern is interpreted
/// as the end of a line.
///
/// This function writes each line in `data` to [`std::io::Stdout`] in
/// reverse.
///
/// # Errors
///
/// If there is a problem writing to `stdout`, then this function
/// returns [`std::io::Error`].
fn buffer_tac_regex(
    data: &[u8],
    pattern: &regex::bytes::Regex,
    before: bool,
) -> std::io::Result<()> {
    let out = stdout();
    let mut out = BufWriter::new(out.lock());

    // The index of the line separator for the current line.
    //
    // As we scan through the `data` from right to left, we update this
    // variable each time we find a new line separator. We restrict our
    // regular expression search to only those bytes up to the line
    // separator.
    let mut this_line_end = data.len();

    // The index of the start of the next line in the `data`.
    //
    // As we scan through the `data` from right to left, we update this
    // variable each time we find a new line.
    //
    // If `before` is `true`, then each line starts immediately before
    // the line separator. Otherwise, each line starts immediately after
    // the line separator.
    let mut following_line_start = data.len();

    // Iterate over each byte in the buffer in reverse. When we find a
    // line separator, write the line to stdout.
    //
    // The `before` flag controls whether the line separator appears at
    // the end of the line (as in "abc\ndef\n") or at the beginning of
    // the line (as in "/abc/def").
    for i in (0..data.len()).rev() {
        // Determine if there is a match for `pattern` starting at index
        // `i` in `data`. Only search up to the line ending that was
        // found previously.
        if let Some(match_) = pattern.find_at(&data[..this_line_end], i)
            && match_.start() == i
        {
            // Record this index as the ending of the current line.
            this_line_end = i;

            // The length of the match (that is, the line separator), in bytes.
            let slen = match_.end() - match_.start();

            if before {
                out.write_all(&data[i..following_line_start])?;
                following_line_start = i;
            } else {
                out.write_all(&data[i + slen..following_line_start])?;
                following_line_start = i + slen;
            }
        }
    }

    // After the loop terminates, write whatever bytes are remaining at
    // the beginning of the buffer.
    out.write_all(&data[0..following_line_start])?;
    out.flush()?;
    Ok(())
}

/// Write lines from `data` to stdout in reverse.
///
/// This function writes to [`stdout`] each line appearing in `data`,
/// starting with the last line and ending with the first line. The
/// `separator` parameter defines what characters to use as a line
/// separator.
///
/// If `before` is `false`, then this function assumes that the
/// `separator` appears at the end of each line, as in `"abc\ndef\n"`.
/// If `before` is `true`, then this function assumes that the
/// `separator` appears at the beginning of each line, as in
/// `"/abc/def"`.
fn buffer_tac(data: &[u8], before: bool, separator: &OsStr) -> std::io::Result<()> {
    let out = stdout();
    let mut out = BufWriter::new(out.lock());

    // The number of bytes in the line separator.
    let slen = separator.len();

    // The index of the start of the next line in the `data`.
    //
    // As we scan through the `data` from right to left, we update this
    // variable each time we find a new line.
    //
    // If `before` is `true`, then each line starts immediately before
    // the line separator. Otherwise, each line starts immediately after
    // the line separator.
    let mut following_line_start = data.len();

    // Iterate over each byte in the buffer in reverse. When we find a
    // line separator, write the line to stdout.
    //
    // The `before` flag controls whether the line separator appears at
    // the end of the line (as in "abc\ndef\n") or at the beginning of
    // the line (as in "/abc/def").
    for i in memmem::rfind_iter(data, separator.as_encoded_bytes()) {
        if before {
            out.write_all(&data[i..following_line_start])?;
            following_line_start = i;
        } else {
            out.write_all(&data[i + slen..following_line_start])?;
            following_line_start = i + slen;
        }
    }

    // After the loop terminates, write whatever bytes are remaining at
    // the beginning of the buffer.
    out.write_all(&data[0..following_line_start])?;
    out.flush()?;
    Ok(())
}

/// Make the regex flavor compatible with `regex` crate
///
/// Concretely:
/// - Toggle escaping of (), |, {}
/// - Escape ^ and $ when not at edges
/// - Leave only ASCII bytes inside []
/// - Escape non-ASCII bytes as `(?-u:\xFF)` outside []
fn translate_regex_flavor(bytes: &[u8]) -> String {
    let mut result = Vec::new();
    let mut i = 0;
    let mut inside_brackets = false;
    let mut prev_was_backslash = false;
    let mut last_byte: Option<u8> = None;

    while let Some(b) = bytes.get(i) {
        let is_escaped = prev_was_backslash;
        prev_was_backslash = false;

        match b {
            _ if inside_brackets && !b.is_ascii() => {
                i += 1;
                continue;
            }
            // Unescape escaped (), |, {} when not inside brackets
            b'\\' if !inside_brackets && !is_escaped => {
                if let Some(next) = bytes.get(i + 1) {
                    if matches!(next, b'(' | b')' | b'|' | b'{' | b'}') {
                        result.push(*next);
                        last_byte = Some(*next);
                        i += 2;
                        continue;
                    }
                }

                result.push(b'\\');
                last_byte = Some(b'\\');
                prev_was_backslash = true;
            }
            // Bracket tracking
            b'[' => {
                inside_brackets = true;
                result.push(*b);
                last_byte = Some(*b);
            }
            b']' => {
                inside_brackets = false;
                result.push(*b);
                last_byte = Some(*b);
            }
            // Escape (), |, {} when not escaped and outside brackets
            b'(' | b')' | b'|' | b'{' | b'}' if !inside_brackets && !is_escaped => {
                result.push(b'\\');
                result.push(*b);
                last_byte = Some(*b);
            }
            b'^' if !inside_brackets && !is_escaped => {
                let is_anchor_position =
                    result.is_empty() || matches!(last_byte, Some(b'(' | b'|'));
                if !is_anchor_position {
                    result.push(b'\\');
                }
                result.push(*b);
                last_byte = Some(*b);
            }
            b'$' if !inside_brackets && !is_escaped => {
                let next_is_anchor_position = match bytes.get(i + 1) {
                    None => true,
                    Some(b')' | b'|') => true,
                    Some(b'\\') => {
                        // Peek two ahead to see if it's \) or \|
                        matches!(bytes.get(i + 2), Some(b')' | b'|'))
                    }
                    _ => false,
                };
                if !next_is_anchor_position {
                    result.push(b'\\');
                }
                result.push(*b);
                last_byte = Some(*b);
            }
            _ if !b.is_ascii() => {
                let _ = write!(result, r"(?-u:\x{b:02x})");
                last_byte = None;
            }
            _ => {
                result.push(*b);
                last_byte = Some(*b);
            }
        }

        i += 1;
    }

    String::from_utf8(result).expect("produces ASCII bytes")
}

#[allow(clippy::cognitive_complexity)]
fn tac(filenames: &[OsString], before: bool, regex: bool, separator: &OsStr) -> UResult<()> {
    // Compile the regular expression pattern if it is provided.
    let maybe_pattern = if regex {
        match regex::bytes::RegexBuilder::new(&translate_regex_flavor(separator.as_encoded_bytes()))
            .multi_line(true)
            .build()
        {
            Ok(p) => Some(p),
            Err(e) => return Err(TacError::InvalidRegex(e).into()),
        }
    } else {
        None
    };

    for filename in filenames {
        let mmap;
        let buf;

        let data: &[u8] = if filename == "-" {
            #[cfg(unix)]
            if uucore::signals::stdin_was_closed() {
                let e: Box<dyn UError> = TacError::ReadError(
                    OsString::from("-"),
                    std::io::Error::from_raw_os_error(libc::EBADF),
                )
                .into();
                show!(e);
                set_exit_code(1);
                continue;
            }
            if let Some(mmap1) = try_mmap_stdin() {
                mmap = mmap1;
                &mmap
            } else {
                // Copy stdin to a temp file (respects TMPDIR), then mmap it.
                // Falls back to Vec buffer if temp file creation fails (e.g., bad TMPDIR).
                match buffer_stdin() {
                    Ok(StdinData::Mmap(mmap1)) => {
                        mmap = mmap1;
                        &mmap
                    }
                    Ok(StdinData::Vec(buf1)) => {
                        buf = buf1;
                        &buf
                    }
                    Err(e) => {
                        show!(TacError::ReadError(OsString::from("stdin"), e));
                        continue;
                    }
                }
            }
        } else {
            let path = Path::new(filename);
            let mut file = match File::open(path) {
                Ok(f) => f,
                Err(e) => {
                    show!(TacError::OpenError(filename.clone(), e));
                    continue;
                }
            };

            if let Some(mmap1) = try_mmap_file(&file) {
                mmap = mmap1;
                &mmap
            } else {
                let mut contents = Vec::new();
                match file.read_to_end(&mut contents) {
                    Ok(_) => {
                        buf = contents;
                        &buf
                    }
                    Err(e) => {
                        show!(TacError::ReadError(filename.clone(), e));
                        continue;
                    }
                }
            }
        };

        // Select the appropriate `tac` algorithm based on whether the
        // separator is given as a regular expression or a fixed string.
        let result = match maybe_pattern {
            Some(ref pattern) => buffer_tac_regex(data, pattern, before),
            None => buffer_tac(data, before, separator),
        };

        // If there is any error in writing the output, terminate immediately.
        if let Err(e) = result {
            return Err(TacError::WriteError(e).into());
        }
    }
    Ok(())
}

fn try_mmap_stdin() -> Option<Mmap> {
    // SAFETY: If the file is truncated while we map it, SIGBUS will be raised
    // and our process will be terminated, thus preventing access of invalid memory.
    let mmap = unsafe { Mmap::map(&stdin()).ok()? };
    // On Windows, mmap on a pipe handle can "succeed" but return 0 bytes
    // (the file size of a pipe is reported as 0). When that happens, return
    // None so we fall through to buffer_stdin() which reads the pipe properly.
    if mmap.is_empty() { None } else { Some(mmap) }
}

enum StdinData {
    Mmap(Mmap),
    Vec(Vec<u8>),
}

/// Copy stdin to a temp file, then memory-map it.
/// Falls back to reading directly into memory if temp file creation fails.
fn buffer_stdin() -> std::io::Result<StdinData> {
    // Try to create a temp file (respects TMPDIR)
    if let Ok(mut tmp) = tempfile::tempfile() {
        // Temp file created - copy stdin to it, then read back
        copy(&mut stdin(), &mut tmp)?;
        // SAFETY: If the file is truncated while we map it, SIGBUS will be raised
        // and our process will be terminated, thus preventing access of invalid memory.
        let mmap = unsafe { Mmap::map(&tmp)? };
        Ok(StdinData::Mmap(mmap))
    } else {
        // Fall back to reading directly into memory (e.g., bad TMPDIR)
        let mut buf = Vec::new();
        stdin().read_to_end(&mut buf)?;
        Ok(StdinData::Vec(buf))
    }
}

fn try_mmap_file(file: &File) -> Option<Mmap> {
    // SAFETY: If the file is truncated while we map it, SIGBUS will be raised
    // and our process will be terminated, thus preventing access of invalid memory.
    unsafe { Mmap::map(file).ok() }
}

#[cfg(test)]
mod tests_hybrid_flavor {
    use super::translate_regex_flavor;

    #[test]
    fn test_grouping_and_alternation() {
        assert_eq!(translate_regex_flavor(br"\(abc\)"), r"(abc)");

        assert_eq!(translate_regex_flavor(br"(abc)"), r"\(abc\)");

        assert_eq!(translate_regex_flavor(br"a\|b"), r"a|b");

        assert_eq!(translate_regex_flavor(br"a|b"), r"a\|b");
    }

    #[test]
    fn test_quantifiers() {
        assert_eq!(translate_regex_flavor(b"a+"), "a+");

        assert_eq!(translate_regex_flavor(b"a*"), "a*");

        assert_eq!(translate_regex_flavor(b"a?"), "a?");

        assert_eq!(translate_regex_flavor(br"a\+"), r"a\+");

        assert_eq!(translate_regex_flavor(br"a\*"), r"a\*");

        assert_eq!(translate_regex_flavor(br"a\?"), r"a\?");
    }

    #[test]
    fn test_intervals() {
        assert_eq!(translate_regex_flavor(br"a\{1,3\}"), r"a{1,3}");

        assert_eq!(translate_regex_flavor(br"a{1,3}"), r"a\{1,3\}");
    }

    #[test]
    fn test_anchors_context() {
        assert_eq!(translate_regex_flavor(br"^abc$"), r"^abc$");

        assert_eq!(translate_regex_flavor(br"a^b"), r"a\^b");
        assert_eq!(translate_regex_flavor(br"a$b"), r"a\$b");

        // Anchors inside groups (reset by \(...\) regardless of position)
        assert_eq!(translate_regex_flavor(br"\(^abc\)"), r"(^abc)");
        assert_eq!(translate_regex_flavor(br"z\(^abc\)"), r"z(^abc)");
        assert_eq!(translate_regex_flavor(br"\(abc$\)"), r"(abc$)");
        assert_eq!(translate_regex_flavor(br"\(abc$\)z"), r"(abc$)z");

        // Anchors inside alternation (reset by \| regardless of position)
        assert_eq!(translate_regex_flavor(br"^a\|^b"), r"^a|^b");
        assert_eq!(translate_regex_flavor(br"x\|^b"), r"x|^b");
        assert_eq!(translate_regex_flavor(br"a$\|b$"), r"a$|b$");
    }

    #[test]
    fn test_character_classes() {
        assert_eq!(translate_regex_flavor(br"[a-z]"), r"[a-z]");

        assert_eq!(translate_regex_flavor(br"[.]"), r"[.]");
        assert_eq!(translate_regex_flavor(br"[+]"), r"[+]");

        assert_eq!(translate_regex_flavor(br"[]abc]"), r"[]abc]");

        assert_eq!(translate_regex_flavor(br"[^]abc]"), r"[^]abc]");
    }

    #[test]
    fn test_complex_strings() {
        assert_eq!(translate_regex_flavor(br"(\d+)[+*]"), r"\(\d+\)[+*]");

        assert_eq!(translate_regex_flavor(br"\(\d+\)\{2\}"), r"(\d+){2}");
    }

    #[test]
    fn test_edge_cases() {
        assert_eq!(translate_regex_flavor(br"abc\"), r"abc\");

        assert_eq!(translate_regex_flavor(br"\\"), r"\\");

        assert_eq!(translate_regex_flavor(br"\^"), r"\^");
    }
}
