// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) INFTY MULT PSKIP accum aftertab beforetab breakwords fmt's formatline linebreak linebreaking linebreaks linelen maxlength minlength nchars noformat noformatline ostream overlen parasplit plass pmatch poffset posn powf prefixindent punct signum slen sstart tabwidth tlen underlen winfo wlen wordlen wordsplits xanti xprefix

use std::io::BufRead;
use std::iter::Peekable;
use std::slice::Iter;
use unicode_width::UnicodeWidthChar;

use crate::FileOrStdReader;
use crate::FmtOptions;

fn char_width(c: char) -> usize {
    if (c as usize) < 0xA0 {
        // if it is ASCII, call it exactly 1 wide (including control chars)
        // calling control chars' widths 1 is consistent with OpenBSD fmt
        1
    } else {
        // otherwise, get the unicode width
        // note that we shouldn't actually get None here because only c < 0xA0
        // can return None, but for safety and future-proofing we do it this way
        UnicodeWidthChar::width(c).unwrap_or(1)
    }
}

/// Return the UTF-8 sequence length implied by a leading byte, or `None` if invalid.
fn utf8_char_width(byte: u8) -> Option<usize> {
    // UTF-8 leading-byte ranges per Unicode Standard, Ch. 3, Table 3-7 and RFC 3629.
    // 00..7F => 1 byte; C2..DF => 2 bytes; E0..EF => 3 bytes; F0..F4 => 4 bytes.
    // Disallowed bytes include C0..C1 and F5..FF.
    const ASCII_MAX: u8 = 0x7F;
    const TWO_BYTE_START: u8 = 0xC2;
    const TWO_BYTE_END: u8 = 0xDF;
    const THREE_BYTE_START: u8 = 0xE0;
    const THREE_BYTE_END: u8 = 0xEF;
    const FOUR_BYTE_START: u8 = 0xF0;
    const FOUR_BYTE_END: u8 = 0xF4; // up to U+10FFFF

    if byte <= ASCII_MAX {
        return Some(1);
    }
    if (TWO_BYTE_START..=TWO_BYTE_END).contains(&byte) {
        return Some(2);
    }
    if (THREE_BYTE_START..=THREE_BYTE_END).contains(&byte) {
        return Some(3);
    }
    if (FOUR_BYTE_START..=FOUR_BYTE_END).contains(&byte) {
        return Some(4);
    }
    None
}

/// Decode a UTF-8 character starting at `start`, returning the char and bytes consumed.
fn decode_char(bytes: &[u8], start: usize) -> (Option<char>, usize) {
    let Some(&first) = bytes.get(start) else {
        return (None, 1);
    };
    if first < 0x80 {
        return (Some(first as char), 1);
    }

    let Some(width) = utf8_char_width(first) else {
        return (None, 1);
    };

    if start + width > bytes.len() {
        return (None, 1);
    }

    match std::str::from_utf8(&bytes[start..start + width]) {
        Ok(s) => (s.chars().next(), width),
        Err(_) => (None, 1),
    }
}

struct DecodedCharInfo {
    ch: Option<char>,
    consumed: usize,
    width: usize,
    is_ascii: bool,
}

fn decode_char_info(bytes: &[u8], start: usize) -> DecodedCharInfo {
    let (ch, consumed) = decode_char(bytes, start);
    let (width, is_ascii) = match ch {
        Some(c) => (char_width(c), c.is_ascii()),
        None => (1, false),
    };
    DecodedCharInfo {
        ch,
        consumed,
        width,
        is_ascii,
    }
}

/// Compute display width for a UTF-8 byte slice, treating invalid bytes as width 1.
fn byte_display_width(bytes: &[u8]) -> usize {
    let mut width = 0;
    let mut idx = 0;
    while idx < bytes.len() {
        let info = decode_char_info(bytes, idx);
        width += info.width;
        idx += info.consumed;
    }
    width
}

/// GNU fmt has a more restrictive definition of whitespace than Unicode.
/// It only considers ASCII whitespace characters (space, tab, newline, etc.)
/// and excludes many Unicode whitespace characters like non-breaking spaces.
fn is_fmt_whitespace(c: char) -> bool {
    // Only ASCII whitespace characters are considered whitespace in GNU fmt
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0B' | '\x0C')
}

fn is_fmt_whitespace_byte(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

// lines with PSKIP, lacking PREFIX, or which are entirely blank are
// NoFormatLines; otherwise, they are FormatLines
#[derive(Debug)]
pub enum Line {
    FormatLine(FileLine),
    NoFormatLine(Vec<u8>, bool),
}

impl Line {
    /// when we know that it's a [`Line::FormatLine`], as in the [`ParagraphStream`] iterator
    fn get_formatline(self) -> FileLine {
        match self {
            Self::FormatLine(fl) => fl,
            Self::NoFormatLine(..) => panic!("Found NoFormatLine when expecting FormatLine"),
        }
    }

    /// when we know that it's a [`Line::NoFormatLine`], as in the [`ParagraphStream`] iterator
    fn get_noformatline(self) -> (Vec<u8>, bool) {
        match self {
            Self::NoFormatLine(s, b) => (s, b),
            Self::FormatLine(..) => panic!("Found FormatLine when expecting NoFormatLine"),
        }
    }
}

/// Each line's prefix has to be considered to know whether to merge it with
/// the next line or not
#[derive(Debug)]
pub struct FileLine {
    line: Vec<u8>,
    /// The end of the indent, always the start of the text
    indent_end: usize,
    /// The end of the PREFIX's indent, that is, the spaces before the prefix
    prefix_indent_end: usize,
    /// Display length of indent taking into account tabs
    indent_len: usize,
    /// PREFIX indent length taking into account tabs
    prefix_len: usize,
}

/// Iterator that produces a stream of Lines from a file
pub struct FileLines<'a> {
    opts: &'a FmtOptions,
    reader: &'a mut FileOrStdReader,
}

impl FileLines<'_> {
    fn new<'b>(opts: &'b FmtOptions, reader: &'b mut FileOrStdReader) -> FileLines<'b> {
        FileLines { opts, reader }
    }

    /// returns true if this line should be formatted
    fn match_prefix(&self, line: &[u8]) -> (bool, usize) {
        let Some(prefix) = &self.opts.prefix else {
            return (true, 0);
        };

        FileLines::match_prefix_generic(prefix.as_bytes(), line, self.opts.xprefix)
    }

    /// returns true if this line should be formatted
    fn match_anti_prefix(&self, line: &[u8]) -> bool {
        let Some(anti_prefix) = &self.opts.anti_prefix else {
            return true;
        };

        match FileLines::match_prefix_generic(anti_prefix.as_bytes(), line, self.opts.xanti_prefix)
        {
            (true, _) => false,
            (_, _) => true,
        }
    }

    fn match_prefix_generic(pfx: &[u8], line: &[u8], exact: bool) -> (bool, usize) {
        if line.starts_with(pfx) {
            return (true, 0);
        }

        if !exact {
            let mut i = 0;
            while i < line.len() {
                if line[i..].starts_with(pfx) {
                    return (true, i);
                } else if !is_fmt_whitespace_byte(line[i]) {
                    break;
                }
                i += 1;
            }
        }

        (false, 0)
    }

    fn compute_indent(&self, bytes: &[u8], prefix_end: usize) -> (usize, usize, usize) {
        let mut prefix_len = 0;
        let mut indent_len = 0;
        let mut indent_end = bytes.len();
        let mut idx = 0;
        while idx < bytes.len() {
            if idx == prefix_end {
                // we found the end of the prefix, so this is the printed length of the prefix here
                prefix_len = indent_len;
            }

            let byte = bytes[idx];
            if idx >= prefix_end && !is_fmt_whitespace_byte(byte) {
                indent_end = idx;
                break;
            }

            if byte == b'\t' {
                indent_len = (indent_len / self.opts.tabwidth + 1) * self.opts.tabwidth;
                idx += 1;
                continue;
            }

            let info = decode_char_info(bytes, idx);
            indent_len += info.width;
            idx += info.consumed;
        }
        if indent_end == bytes.len() {
            indent_end = idx;
        }
        (indent_end, prefix_len, indent_len)
    }
}

impl Iterator for FileLines<'_> {
    type Item = Line;

    fn next(&mut self) -> Option<Line> {
        let mut buf = Vec::new();
        match self.reader.read_until(b'\n', &mut buf) {
            Ok(0) => return None,
            Ok(_) => {}
            Err(_) => return None,
        }
        if buf.ends_with(b"\n") {
            buf.pop();
            if buf.ends_with(b"\r") {
                buf.pop();
            }
        }
        let n = buf;

        // if this line is entirely whitespace,
        // emit a blank line
        // Err(true) indicates that this was a linebreak,
        // which is important to know when detecting mail headers
        if n.iter().all(|&b| is_fmt_whitespace_byte(b)) {
            return Some(Line::NoFormatLine(Vec::new(), true));
        }

        let (pmatch, poffset) = self.match_prefix(&n[..]);

        // if this line does not match the prefix,
        // emit the line unprocessed and iterate again
        if !pmatch {
            return Some(Line::NoFormatLine(n, false));
        }

        // if the line matches the prefix, but is blank after,
        // don't allow lines to be combined through it (that is,
        // treat it like a blank line, except that since it's
        // not truly blank we will not allow mail headers on the
        // following line)
        if pmatch
            && n[poffset + self.opts.prefix.as_ref().map_or(0, String::len)..]
                .iter()
                .all(|&b| is_fmt_whitespace_byte(b))
        {
            return Some(Line::NoFormatLine(n, false));
        }

        // skip if this line matches the anti_prefix
        // (NOTE definition of match_anti_prefix is TRUE if we should process)
        if !self.match_anti_prefix(&n[..]) {
            return Some(Line::NoFormatLine(n, false));
        }

        // figure out the indent, prefix, and prefixindent ending points
        let prefix_end = poffset + self.opts.prefix.as_ref().map_or(0, String::len);
        let (indent_end, prefix_len, indent_len) = self.compute_indent(&n[..], prefix_end);

        Some(Line::FormatLine(FileLine {
            line: n,
            indent_end,
            prefix_indent_end: poffset,
            indent_len,
            prefix_len,
        }))
    }
}

/// A paragraph : a collection of [`FileLines`] that are to be formatted
/// plus info about the paragraph's indentation
///
/// We retain the raw bytes from the [`FileLine`]; the other info
/// is only there to help us in deciding how to merge lines into Paragraphs
#[derive(Debug)]
pub struct Paragraph {
    /// the lines of the file
    lines: Vec<Vec<u8>>,
    /// string representing the init, that is, the first line's indent
    pub init_str: Vec<u8>,
    /// printable length of the init string considering TABWIDTH
    pub init_len: usize,
    /// byte location of end of init in first line buffer
    init_end: usize,
    /// string representing indent
    pub indent_str: Vec<u8>,
    /// length of above
    pub indent_len: usize,
    /// byte location of end of indent (in crown and tagged mode, only applies to 2nd line and onward)
    indent_end: usize,
    /// we need to know if this is a mail header because we do word splitting differently in that case
    pub mail_header: bool,
}

/// An iterator producing a stream of paragraphs from a stream of lines
/// given a set of options.
pub struct ParagraphStream<'a> {
    lines: Peekable<FileLines<'a>>,
    next_mail: bool,
    opts: &'a FmtOptions,
}

impl ParagraphStream<'_> {
    pub fn new<'b>(opts: &'b FmtOptions, reader: &'b mut FileOrStdReader) -> ParagraphStream<'b> {
        let lines = FileLines::new(opts, reader).peekable();
        // at the beginning of the file, we might find mail headers
        ParagraphStream {
            lines,
            next_mail: true,
            opts,
        }
    }

    /// Detect RFC822 mail header
    fn is_mail_header(line: &FileLine) -> bool {
        // a mail header begins with either "From " (envelope sender line)
        // or with a sequence of printable ASCII chars (33 to 126, inclusive,
        // except colon) followed by a colon.
        if line.indent_end > 0 {
            false
        } else {
            let l_slice = &line.line[..];
            if l_slice.starts_with(b"From ") {
                true
            } else {
                let Some(colon_posn) = l_slice.iter().position(|&b| b == b':') else {
                    return false;
                };

                // header field must be nonzero length
                if colon_posn == 0 {
                    return false;
                }

                l_slice[..colon_posn]
                    .iter()
                    .all(|&b| (33..=126).contains(&(b as usize)) && b != b':')
            }
        }
    }
}

impl Iterator for ParagraphStream<'_> {
    type Item = Result<Paragraph, Vec<u8>>;

    #[allow(clippy::cognitive_complexity)]
    fn next(&mut self) -> Option<Result<Paragraph, Vec<u8>>> {
        // return a NoFormatLine in an Err; it should immediately be output
        let noformat = match self.lines.peek()? {
            Line::FormatLine(_) => false,
            Line::NoFormatLine(_, _) => true,
        };

        // found a NoFormatLine, immediately dump it out
        if noformat {
            let (s, nm) = self.lines.next().unwrap().get_noformatline();
            self.next_mail = nm;
            return Some(Err(s));
        }

        // found a FormatLine, now build a paragraph
        let mut init_str = Vec::new();
        let mut init_end = 0;
        let mut init_len = 0;
        let mut indent_str = Vec::new();
        let mut indent_end = 0;
        let mut indent_len = 0;
        let mut prefix_len = 0;
        let mut prefix_indent_end = 0;
        let mut p_lines = Vec::new();

        let mut in_mail = false;
        let mut second_done = false; // for when we use crown or tagged mode
        while let Some(Line::FormatLine(fl)) = self.lines.peek() {
            // peek ahead
            // need to explicitly force fl out of scope before we can call self.lines.next()
            if p_lines.is_empty() {
                // first time through the loop, get things set up
                // detect mail header
                if self.opts.mail && self.next_mail && ParagraphStream::is_mail_header(fl) {
                    in_mail = true;
                    // there can't be any indent or prefixindent because otherwise is_mail_header
                    // would fail since there cannot be any whitespace before the colon in a
                    // valid header field
                    indent_str.extend_from_slice(b"  ");
                    indent_len = 2;
                } else {
                    if self.opts.crown || self.opts.tagged {
                        init_str.extend_from_slice(&fl.line[..fl.indent_end]);
                        init_len = fl.indent_len;
                        init_end = fl.indent_end;
                    } else {
                        second_done = true;
                    }

                    // these will be overwritten in the 2nd line of crown or tagged mode, but
                    // we are not guaranteed to get to the 2nd line, e.g., if the next line
                    // is a NoFormatLine or None. Thus, we set sane defaults the 1st time around
                    indent_str.extend_from_slice(&fl.line[..fl.indent_end]);
                    indent_len = fl.indent_len;
                    indent_end = fl.indent_end;

                    // save these to check for matching lines
                    prefix_len = fl.prefix_len;
                    prefix_indent_end = fl.prefix_indent_end;

                    // in tagged mode, add 4 spaces of additional indenting by default
                    // (gnu fmt's behavior is different: it seems to find the closest column to
                    // indent_end that is divisible by 3. But honestly that behavior seems
                    // pretty arbitrary.
                    // Perhaps a better default would be 1 TABWIDTH? But ugh that's so big.
                    if self.opts.tagged {
                        indent_str.extend_from_slice(b"    ");
                        indent_len += 4;
                    }
                }
            } else if in_mail {
                // lines following mail headers must begin with spaces
                if fl.indent_end == 0 || (self.opts.prefix.is_some() && fl.prefix_indent_end == 0) {
                    break; // this line does not begin with spaces
                }
            } else if !second_done {
                // now we have enough info to handle crown margin and tagged mode

                // in both crown and tagged modes we require that prefix_len is the same
                if prefix_len != fl.prefix_len || prefix_indent_end != fl.prefix_indent_end {
                    break;
                }

                // in tagged mode, indent has to be *different* on following lines
                if self.opts.tagged
                    && indent_len - 4 == fl.indent_len
                    && indent_end == fl.indent_end
                {
                    break;
                }

                // this is part of the same paragraph, get the indent info from this line
                indent_str.clear();
                indent_str.extend_from_slice(&fl.line[..fl.indent_end]);
                indent_len = fl.indent_len;
                indent_end = fl.indent_end;

                second_done = true;
            } else {
                // detect mismatch
                if indent_end != fl.indent_end
                    || prefix_indent_end != fl.prefix_indent_end
                    || indent_len != fl.indent_len
                    || prefix_len != fl.prefix_len
                {
                    break;
                }
            }

            p_lines.push(self.lines.next().unwrap().get_formatline().line);

            // when we're in split-only mode, we never join lines, so stop here
            if self.opts.split_only {
                break;
            }
        }

        // if this was a mail header, then the next line can be detected as one. Otherwise, it cannot.
        // NOTE next_mail is true at ParagraphStream instantiation, and is set to true after a blank
        // NoFormatLine.
        self.next_mail = in_mail;

        Some(Ok(Paragraph {
            lines: p_lines,
            init_str,
            init_len,
            init_end,
            indent_str,
            indent_len,
            indent_end,
            mail_header: in_mail,
        }))
    }
}

pub struct ParaWords<'a> {
    opts: &'a FmtOptions,
    para: &'a Paragraph,
    words: Vec<WordInfo<'a>>,
}

impl<'a> ParaWords<'a> {
    pub fn new(opts: &'a FmtOptions, para: &'a Paragraph) -> Self {
        let mut pw = ParaWords {
            opts,
            para,
            words: Vec::new(),
        };
        pw.create_words();
        pw
    }

    fn create_words(&mut self) {
        if self.para.mail_header {
            // no extra spacing for mail headers; always exactly 1 space
            // safe to trim_start on every line of a mail header, since the
            // first line is guaranteed not to have any spaces
            self.words.extend(
                self.para
                    .lines
                    .iter()
                    .flat_map(|x| {
                        x.split(|b| is_fmt_whitespace_byte(*b))
                            .filter(|segment| !segment.is_empty())
                    })
                    .map(|x| WordInfo {
                        word: x,
                        word_start: 0,
                        word_nchars: byte_display_width(x),
                        before_tab: None,
                        after_tab: 0,
                        sentence_start: false,
                        ends_punct: false,
                        new_line: false,
                    }),
            );
        } else {
            // first line
            self.words.extend(if self.opts.crown || self.opts.tagged {
                // crown and tagged mode has the "init" in the first line, so slice from there
                WordSplit::new(self.opts, &self.para.lines[0][self.para.init_end..])
            } else {
                // otherwise we slice from the indent
                WordSplit::new(self.opts, &self.para.lines[0][self.para.indent_end..])
            });

            if self.para.lines.len() > 1 {
                let indent_end = self.para.indent_end;
                let opts = self.opts;
                self.words.extend(
                    self.para
                        .lines
                        .iter()
                        .skip(1)
                        .flat_map(|x| WordSplit::new(opts, &x[indent_end..])),
                );
            }
        }
    }

    pub fn words(&'a self) -> Iter<'a, WordInfo<'a>> {
        self.words.iter()
    }
}

struct WordSplit<'a> {
    opts: &'a FmtOptions,
    bytes: &'a [u8],
    length: usize,
    position: usize,
    prev_punct: bool,
}

impl WordSplit<'_> {
    fn analyze_tabs(&self, bytes: &[u8]) -> (Option<usize>, usize, Option<usize>) {
        let mut beforetab = None;
        let mut aftertab = 0;
        let mut word_start = None;
        for (idx, b) in bytes.iter().enumerate() {
            if !is_fmt_whitespace_byte(*b) {
                word_start = Some(idx);
                break;
            } else if *b == b'\t' {
                if beforetab.is_none() {
                    beforetab = Some(aftertab);
                    aftertab = 0;
                } else {
                    aftertab = (aftertab / self.opts.tabwidth + 1) * self.opts.tabwidth;
                }
            } else {
                aftertab += 1;
            }
        }
        (beforetab, aftertab, word_start)
    }

    fn new<'b>(opts: &'b FmtOptions, bytes: &'b [u8]) -> WordSplit<'b> {
        let start = bytes
            .iter()
            .position(|&b| !is_fmt_whitespace_byte(b))
            .unwrap_or(bytes.len());
        let trimmed = &bytes[start..];
        WordSplit {
            opts,
            bytes: trimmed,
            length: trimmed.len(),
            position: 0,
            prev_punct: false,
        }
    }

    fn is_punctuation_byte(b: u8) -> bool {
        matches!(b, b'!' | b'.' | b'?')
    }

    fn scan_word_end(&self, word_start: usize) -> (usize, usize, Option<u8>) {
        let mut word_nchars = 0;
        let mut idx = word_start;
        let mut last_ascii = None;
        while idx < self.length {
            let info = decode_char_info(self.bytes, idx);
            let is_whitespace = info.is_ascii && info.ch.is_some_and(is_fmt_whitespace);
            if is_whitespace {
                break;
            }
            word_nchars += info.width;
            if info.is_ascii {
                last_ascii = info.ch.map(|c| c as u8);
            } else {
                last_ascii = None;
            }
            idx += info.consumed;
        }
        (idx, word_nchars, last_ascii)
    }
}

pub struct WordInfo<'a> {
    pub word: &'a [u8],
    pub word_start: usize,
    pub word_nchars: usize,
    pub before_tab: Option<usize>,
    pub after_tab: usize,
    pub sentence_start: bool,
    pub ends_punct: bool,
    pub new_line: bool,
}

// returns (&str, is_start_of_sentence)
impl<'a> Iterator for WordSplit<'a> {
    type Item = WordInfo<'a>;

    fn next(&mut self) -> Option<WordInfo<'a>> {
        if self.position >= self.length {
            return None;
        }

        let old_position = self.position;
        let new_line = old_position == 0;

        // find the start of the next word, and record if we find a tab character
        let (before_tab, after_tab, word_start) =
            if let (b, a, Some(s)) = self.analyze_tabs(&self.bytes[old_position..]) {
                (b, a, s + old_position)
            } else {
                self.position = self.length;
                return None;
            };

        // find the beginning of the next whitespace
        // note that this preserves the invariant that self.position
        // points to whitespace character OR end of string
        let (next_position, word_nchars, last_ascii) = self.scan_word_end(word_start);
        self.position = next_position;

        let word_start_relative = word_start - old_position;
        // if the previous sentence was punctuation and this sentence has >2 whitespace or one tab, is a new sentence.
        let is_start_of_sentence =
            self.prev_punct && (before_tab.is_some() || word_start_relative > 1);

        // now record whether this word ends in punctuation
        let ends_punct = last_ascii.is_some_and(WordSplit::is_punctuation_byte);
        self.prev_punct = ends_punct;

        let (word, word_start_relative, before_tab, after_tab) = if self.opts.uniform {
            (&self.bytes[word_start..self.position], 0, None, 0)
        } else {
            (
                &self.bytes[old_position..self.position],
                word_start_relative,
                before_tab,
                after_tab,
            )
        };

        Some(WordInfo {
            word,
            word_start: word_start_relative,
            word_nchars,
            before_tab,
            after_tab,
            sentence_start: is_start_of_sentence,
            ends_punct,
            new_line,
        })
    }
}
