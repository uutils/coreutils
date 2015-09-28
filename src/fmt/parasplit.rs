/*
 * This file is part of `fmt` from the uutils coreutils package.
 *
 * (c) kwantam <kwantam@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use std::iter::Peekable;
use std::io::{BufRead, Lines};
use std::slice::Iter;
use std::str::CharRange;
use rustc_unicode::str::UnicodeStr;
use unicode_width::UnicodeWidthChar;
use FileOrStdReader;
use FmtOptions;

#[inline(always)]
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

// lines with PSKIP, lacking PREFIX, or which are entirely blank are
// NoFormatLines; otherwise, they are FormatLines
#[derive(Debug)]
pub enum Line {
    FormatLine(FileLine),
    NoFormatLine(String, bool)
}

impl Line {
    // when we know that it's a FormatLine, as in the ParagraphStream iterator
    fn get_formatline(self) -> FileLine {
        match self {
            Line::FormatLine(fl) => fl,
            Line::NoFormatLine(..) => panic!("Found NoFormatLine when expecting FormatLine")
        }
    }

    // when we know that it's a NoFormatLine, as in the ParagraphStream iterator
    fn get_noformatline(self) -> (String, bool) {
        match self {
            Line::NoFormatLine(s, b) => (s, b),
            Line::FormatLine(..) => panic!("Found FormatLine when expecting NoFormatLine")
        }
    }
}

// each line's prefix has to be considered to know whether to merge it with
// the next line or not
#[derive(Debug)]
struct FileLine {
    line       : String,
    indent_end : usize,     // the end of the indent, always the start of the text
    pfxind_end : usize,     // the end of the PREFIX's indent, that is, the spaces before the prefix
    indent_len : usize,     // display length of indent taking into account tabs
    prefix_len : usize,     // PREFIX indent length taking into account tabs
}

// iterator that produces a stream of Lines from a file
pub struct FileLines<'a> {
    opts  : &'a FmtOptions,
    lines : Lines<&'a mut FileOrStdReader>,
}

impl<'a> FileLines<'a> {
    fn new<'b>(opts: &'b FmtOptions, lines: Lines<&'b mut FileOrStdReader>) -> FileLines<'b> {
        FileLines { opts: opts, lines: lines }
    }

    // returns true if this line should be formatted
    fn match_prefix(&self, line: &str) -> (bool, usize) {
        if !self.opts.use_prefix { return (true, 0); }

        FileLines::match_prefix_generic(&self.opts.prefix[..], line, self.opts.xprefix)
    }

    // returns true if this line should be formatted
    fn match_anti_prefix(&self, line: &str) -> bool {
        if !self.opts.use_anti_prefix { return true; }

        match FileLines::match_prefix_generic(&self.opts.anti_prefix[..], line, self.opts.xanti_prefix) {
            (true, _) => false,
            (_   , _) => true
        }
    }

    fn match_prefix_generic(pfx: &str, line: &str, exact: bool) -> (bool, usize) {
        if line.starts_with(pfx) {
            return (true, 0);
        }

        if !exact {
            // we do it this way rather than byte indexing to support unicode whitespace chars
            let mut i = 0;
            while (i < line.len()) && line.char_at(i).is_whitespace() {
                i = match line.char_range_at(i) { CharRange { ch: _ , next: nxi } => nxi };
                if line[i..].starts_with(pfx) {
                    return (true, i);
                }
            }
        }

        (false, 0)
    }

    fn compute_indent(&self, string: &str, prefix_end: usize) -> (usize, usize, usize) {
        let mut prefix_len = 0;
        let mut indent_len = 0;
        let mut indent_end = 0;
        for (os, c) in string.char_indices() {
            if os == prefix_end {
                // we found the end of the prefix, so this is the printed length of the prefix here
                prefix_len = indent_len;
            }

            if (os >= prefix_end) && !c.is_whitespace() {
                // found first non-whitespace after prefix, this is indent_end
                indent_end = os;
                break;
            } else if c == '\t' {
                // compute tab length
                indent_len = (indent_len / self.opts.tabwidth + 1) * self.opts.tabwidth;
            } else {
                // non-tab character
                indent_len += char_width(c);
            }
        }
        (indent_end, prefix_len, indent_len)
    }
}

impl<'a> Iterator for FileLines<'a> {
    type Item = Line;

    fn next(&mut self) -> Option<Line> {
        let n =
            match self.lines.next() {
                Some(t) => match t {
                    Ok(tt) => tt,
                    Err(_) => return None
                },
                None => return None
            };

        // if this line is entirely whitespace,
        // emit a blank line
        // Err(true) indicates that this was a linebreak,
        // which is important to know when detecting mail headers
        if n.is_whitespace() {
            return Some(Line::NoFormatLine("\n".to_string(), true));
        }

        // if this line does not match the prefix,
        // emit the line unprocessed and iterate again
        let (pmatch, poffset) = self.match_prefix(&n[..]);
        if !pmatch {
            return Some(Line::NoFormatLine(n, false));
        } else if n[poffset + self.opts.prefix.len()..].is_whitespace() {
            // if the line matches the prefix, but is blank after,
            // don't allow lines to be combined through it (that is,
            // treat it like a blank line, except that since it's
            // not truly blank we will not allow mail headers on the
            // following line)
            return Some(Line::NoFormatLine(n, false));
        }

        // skip if this line matches the anti_prefix
        // (NOTE definition of match_anti_prefix is TRUE if we should process)
        if !self.match_anti_prefix(&n[..]) {
            return Some(Line::NoFormatLine(n, false));
        }

        // figure out the indent, prefix, and prefixindent ending points
        let prefix_end = poffset + self.opts.prefix.len();
        let (indent_end, prefix_len, indent_len) = self.compute_indent(&n[..], prefix_end);

        Some(Line::FormatLine(FileLine {
            line       : n,
            indent_end : indent_end,
            pfxind_end : poffset,
            indent_len : indent_len,
            prefix_len : prefix_len
        }))
    }
}

// a paragraph : a collection of FileLines that are to be formatted
// plus info about the paragraph's indentation
// (but we only retain the String from the FileLine; the other info
// is only there to help us in deciding how to merge lines into Paragraphs
#[derive(Debug)]
pub struct Paragraph {
        lines       : Vec<String>,  // the lines of the file
    pub init_str    : String,       // string representing the init, that is, the first line's indent
    pub init_len    : usize,        // printable length of the init string considering TABWIDTH
        init_end    : usize,        // byte location of end of init in first line String
    pub indent_str  : String,       // string representing indent
    pub indent_len  : usize,        // length of above
        indent_end  : usize,        // byte location of end of indent (in crown and tagged mode, only applies to 2nd line and onward)
    pub mail_header : bool          // we need to know if this is a mail header because we do word splitting differently in that case
}

// an iterator producing a stream of paragraphs from a stream of lines
// given a set of options.
pub struct ParagraphStream<'a> {
    lines     : Peekable<FileLines<'a>>,
    next_mail : bool,
    opts      : &'a FmtOptions,
}

impl<'a> ParagraphStream<'a> {
    pub fn new<'b>(opts: &'b FmtOptions, reader: &'b mut FileOrStdReader) -> ParagraphStream<'b> {
        let lines = FileLines::new(opts, reader.lines()).peekable();
        // at the beginning of the file, we might find mail headers
        ParagraphStream { lines: lines, next_mail: true, opts: opts }
    }

    // detect RFC822 mail header
    fn is_mail_header(line: &FileLine) -> bool {
        // a mail header begins with either "From " (envelope sender line)
        // or with a sequence of printable ASCII chars (33 to 126, inclusive,
        // except colon) followed by a colon.
        if line.indent_end > 0 {
            false
        } else {
            let l_slice = &line.line[..];
            if l_slice.starts_with("From ") {
                true
            } else {
                let colon_posn =
                    match l_slice.find(':') {
                        Some(n) => n,
                        None => return false
                    };

                // header field must be nonzero length
                if colon_posn == 0 { return false; }

                return l_slice[..colon_posn].chars().all(|x| match x as usize {
                    y if y < 33 || y > 126 => false,
                    _ => true
                });
            }
        }
    }
}

impl<'a> Iterator for ParagraphStream<'a> {
    type Item = Result<Paragraph, String>;

    fn next(&mut self) -> Option<Result<Paragraph, String>> {
        // return a NoFormatLine in an Err; it should immediately be output
        let noformat =
            match self.lines.peek() {
                None => return None,
                Some(l) => match l {
                    &Line::FormatLine(_) => false,
                    &Line::NoFormatLine(_, _) => true
                }
            };

        // found a NoFormatLine, immediately dump it out
        if noformat {
            let (s, nm) = self.lines.next().unwrap().get_noformatline();
            self.next_mail = nm;
            return Some(Err(s));
        }

        // found a FormatLine, now build a paragraph
        let mut init_str = String::new();
        let mut init_end = 0;
        let mut init_len = 0;
        let mut indent_str = String::new();
        let mut indent_end = 0;
        let mut indent_len = 0;
        let mut prefix_len = 0;
        let mut pfxind_end = 0;
        let mut p_lines = Vec::new();

        let mut in_mail = false;
        let mut second_done = false;    // for when we use crown or tagged mode
        loop {
            {   // peek ahead
            // need to explicitly force fl out of scope before we can call self.lines.next()
                let fl =
                    match self.lines.peek() {
                        None => break,
                        Some(l) => {
                            match l {
                                &Line::FormatLine(ref x) => x,
                                &Line::NoFormatLine(..) => break
                            }
                        }
                    };

                if p_lines.len() == 0 {
                    // first time through the loop, get things set up
                    // detect mail header
                    if self.opts.mail && self.next_mail && ParagraphStream::is_mail_header(fl) {
                        in_mail = true;
                        // there can't be any indent or pfxind because otherwise is_mail_header would fail
                        // since there cannot be any whitespace before the colon in a valid header field
                        indent_str.push_str("  ");
                        indent_len = 2;
                    } else {
                        if self.opts.crown || self.opts.tagged {
                            init_str.push_str(&fl.line[..fl.indent_end]);
                            init_len = fl.indent_len;
                            init_end = fl.indent_end;
                        } else {
                            second_done = true;
                        }

                        // these will be overwritten in the 2nd line of crown or tagged mode, but
                        // we are not guaranteed to get to the 2nd line, e.g., if the next line
                        // is a NoFormatLine or None. Thus, we set sane defaults the 1st time around
                        indent_str.push_str(&fl.line[..fl.indent_end]);
                        indent_len = fl.indent_len;
                        indent_end = fl.indent_end;

                        // save these to check for matching lines
                        prefix_len = fl.prefix_len;
                        pfxind_end = fl.pfxind_end;

                        // in tagged mode, add 4 spaces of additional indenting by default
                        // (gnu fmt's behavior is different: it seems to find the closest column to
                        // indent_end that is divisible by 3. But honesly that behavior seems
                        // pretty arbitrary.
                        // Perhaps a better default would be 1 TABWIDTH? But ugh that's so big.
                        if self.opts.tagged {
                            indent_str.push_str("    ");
                            indent_len += 4;
                        }
                    }
                } else if in_mail {
                    // lines following mail headers must begin with spaces
                    if fl.indent_end == 0 || (self.opts.use_prefix && fl.pfxind_end == 0) {
                        break;  // this line does not begin with spaces
                    }
                } else if !second_done {
                    // now we have enough info to handle crown margin and tagged mode
                    if prefix_len != fl.prefix_len || pfxind_end != fl.pfxind_end {
                        // in both crown and tagged modes we require that prefix_len is the same
                        break;
                    } else if self.opts.tagged && indent_len - 4 == fl.indent_len && indent_end == fl.indent_end {
                        // in tagged mode, indent has to be *different* on following lines
                        break;
                    } else {
                        // this is part of the same paragraph, get the indent info from this line
                        indent_str.clear();
                        indent_str.push_str(&fl.line[..fl.indent_end]);
                        indent_len = fl.indent_len;
                        indent_end = fl.indent_end;
                    }
                    second_done = true;
                } else {
                    // detect mismatch
                    if indent_end != fl.indent_end || pfxind_end != fl.pfxind_end || indent_len != fl.indent_len || prefix_len != fl.prefix_len {
                        break;
                    }
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
            lines       : p_lines,
            init_str    : init_str,
            init_len    : init_len,
            init_end    : init_end,
            indent_str  : indent_str,
            indent_len  : indent_len,
            indent_end  : indent_end,
            mail_header : in_mail
        }))
    }
}

pub struct ParaWords<'a> {
    opts  : &'a FmtOptions,
    para  : &'a Paragraph,
    words : Vec<WordInfo<'a>>
}

impl<'a> ParaWords<'a> {
    pub fn new<'b>(opts: &'b FmtOptions, para: &'b Paragraph) -> ParaWords<'b> {
        let mut pw = ParaWords { opts: opts, para: para, words: Vec::new() };
        pw.create_words();
        pw
    }

    fn create_words<'r>(&'r mut self) {
        if self.para.mail_header {
            // no extra spacing for mail headers; always exactly 1 space
            // safe to trim_left on every line of a mail header, since the
            // first line is guaranteed not to have any spaces
            self.words.extend(self.para.lines.iter().flat_map(|x| x.split_whitespace()).map(|x| WordInfo {
                word           : x,
                word_start     : 0,
                word_nchars    : x.len(),  // OK for mail headers; only ASCII allowed (unicode is escaped)
                before_tab     : None,
                after_tab      : 0,
                sentence_start : false,
                ends_punct     : false,
                new_line       : false
            }));
        } else {
            // first line
            self.words.extend(
                if self.opts.crown || self.opts.tagged {
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
                    self.para.lines.iter().skip(1).flat_map(|x| WordSplit::new(opts, &x[indent_end..])));
            }
        }
    }

    pub fn words(&'a self) -> Iter<'a, WordInfo<'a>> { return self.words.iter() }
}

struct WordSplit<'a> {
    opts       : &'a FmtOptions,
    string     : &'a str,
    length     : usize,
    position   : usize,
    prev_punct : bool
}

impl<'a> WordSplit<'a> {
    fn analyze_tabs(&self, string: &str) -> (Option<usize>, usize, Option<usize>) {
        // given a string, determine (length before tab) and (printed length after first tab)
        // if there are no tabs, beforetab = -1 and aftertab is the printed length
        let mut beforetab = None;
        let mut aftertab = 0;
        let mut word_start = None;
        for (os, c) in string.char_indices() {
            if !c.is_whitespace() {
                word_start = Some(os);
                break;
            } else if c == '\t' {
                if beforetab == None {
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
}

impl<'a> WordSplit<'a> {
    fn new<'b>(opts: &'b FmtOptions, string: &'b str) -> WordSplit<'b> {
        // wordsplits *must* start at a non-whitespace character
        let trim_string = string.trim_left();
        WordSplit { opts: opts, string: trim_string, length: string.len(), position: 0, prev_punct: false }
    }

    fn is_punctuation(c: char) -> bool {
        match c {
            '!' | '.' | '?' => true,
            _ => false
        }
    }
}

pub struct WordInfo<'a> {
    pub word           : &'a str,
    pub word_start     : usize,
    pub word_nchars    : usize,
    pub before_tab     : Option<usize>,
    pub after_tab      : usize,
    pub sentence_start : bool,
    pub ends_punct     : bool,
    pub new_line       : bool
}

// returns (&str, is_start_of_sentence)
impl<'a> Iterator for WordSplit<'a> {
    type Item = WordInfo<'a>;

    fn next(&mut self) -> Option<WordInfo<'a>> {
        if self.position >= self.length {
            return None
        }

        let old_position = self.position;
        let new_line = old_position == 0;

        // find the start of the next word, and record if we find a tab character
        let (before_tab, after_tab, word_start) = match self.analyze_tabs(&self.string[old_position..]) {
            (b, a, Some(s)) => (b, a, s + old_position),
            (_, _, None) => {
                self.position = self.length;
                return None;
            }
        };

        // find the beginning of the next whitespace
        // note that this preserves the invariant that self.position
        // points to whitespace character OR end of string
        let mut word_nchars = 0;
        self.position =
            match self.string[word_start..]
            .find(|x: char| if !x.is_whitespace() { word_nchars += char_width(x); false } else { true }) {
                None => self.length,
                Some(s) => s + word_start
            };

        let word_start_relative = word_start - old_position;
        // if the previous sentence was punctuation and this sentence has >2 whitespace or one tab, is a new sentence.
        let is_start_of_sentence = self.prev_punct && (before_tab.is_some() || word_start_relative > 1);

        // now record whether this word ends in punctuation
        self.prev_punct = match self.string.char_range_at_reverse(self.position) {
            CharRange { ch, next: _ } => WordSplit::is_punctuation(ch)
        };

        let (word, word_start_relative, before_tab, after_tab) =
            if self.opts.uniform {
                (&self.string[word_start..self.position], 0, None, 0)
            } else {
                (&self.string[old_position..self.position], word_start_relative, before_tab, after_tab)
            };

        Some(WordInfo {
            word           : word,
            word_start     : word_start_relative,
            word_nchars    : word_nchars,
            before_tab     : before_tab,
            after_tab      : after_tab,
            sentence_start : is_start_of_sentence,
            ends_punct     : self.prev_punct,
            new_line       : new_line
        })
    }
}
