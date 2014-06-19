/*
 * This file is part of `fmt` from the uutils coreutils package.
 *
 * (c) kwantam <kwantam@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use core::iter::Peekable;
use std::io::Lines;
use std::slice::Items;
use std::str::CharRange;
use FileOrStdReader;
use FmtOptions;

// lines with PSKIP, lacking PREFIX, or which are entirely blank are
// NoFormatLines; otherwise, they are FormatLines
#[deriving(Show)]
enum Line {
    FormatLine(FileLine),
    NoFormatLine(String, bool)
}

impl Line {
    // when we know that it's a FormatLine, as in the ParagraphStream iterator
    fn get_fileline(self) -> FileLine {
        match self {
            FormatLine(fl) => fl,
            NoFormatLine(..) => fail!("Found NoFormatLine when expecting FormatLine")
        }
    }

    // when we know that it's a NoFormatLine, as in the ParagraphStream iterator
    fn get_noformatline(self) -> (String, bool) {
        match self {
            NoFormatLine(s, b) => (s, b),
            FormatLine(..) => fail!("Found FormatLine when expecting NoFormatLine")
        }
    }
}

// each line's prefix has to be considered to know whether to merge it with
// the next line or not
#[deriving(Show)]
struct FileLine {
    line       : String,
    indent_end : uint,     // the end of the indent, always the start of the text
    prefix_end : uint,     // the end of the PREFIX
    pfxind_end : uint,     // the end of the PREFIX's indent, that is, the spaces before the prefix
    indent_len : uint,     // display length of indent taking into account TABWIDTH
    pfxind_len : uint,     // PREFIX indent length taking into account TABWIDTH
}

// iterator that produces a stream of Lines from a file
struct FileLines<'a> {
    opts  : &'a FmtOptions,
    lines : Lines<'a, FileOrStdReader>,
}

impl<'a> FileLines<'a> {
    fn new<'a>(opts: &'a FmtOptions, lines: Lines<'a, FileOrStdReader>) -> FileLines<'a> {
        FileLines { opts: opts, lines: lines }
    }

    // returns true if this line should be formatted
    fn match_prefix(&self, line: &str) -> (bool, uint) {
        if !self.opts.use_prefix { return (true, 0u); }

        FileLines::match_prefix_generic(self.opts.prefix.as_slice(), line, self.opts.xprefix)
    }

    // returns true if this line should be formatted
    fn match_anti_prefix(&self, line: &str) -> bool {
        if !self.opts.use_anti_prefix { return true; }

        match FileLines::match_prefix_generic(self.opts.anti_prefix.as_slice(), line, self.opts.xanti_prefix) {
            (true, _) => false,
            (_   , _) => true
        }
    }

    fn match_prefix_generic(pfx: &str, line: &str, exact: bool) -> (bool, uint) {
        if line.starts_with(pfx) {
            return (true, 0);
        }

        if !exact {
            // we do it this way rather than byte indexing to support unicode whitespace chars
            let mut i = 0u;
            while (i < line.len()) && line.char_at(i).is_whitespace() {
                i = match line.char_range_at(i) { CharRange { ch: _ , next: nxi } => nxi };
                if line.slice_from(i).starts_with(pfx) {
                    return (true, i);
                }
            }
        }

        (false, 0)
    }

    fn displayed_length(&self, s: &str) -> uint {
        s.char_len() + (self.opts.tabwidth - 1) * s.chars().filter(|x| x == &'\t').count()
    }
}

impl<'a> Iterator<Line> for FileLines<'a> {
    fn next(&mut self) -> Option<Line> {
        let mut n =
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
        if n.as_slice().is_whitespace() {
            return Some(NoFormatLine("\n".to_string(), true));
        }

        // if this line does not match the prefix,
        // emit the line unprocessed and iterate again
        let (pmatch, poffset) = self.match_prefix(n.as_slice());
        if !pmatch {
            return Some(NoFormatLine(n, false));
        }

        // if this line matches the anti_prefix
        // (NOTE definition of match_anti_prefix is TRUE if we should process)
        if !self.match_anti_prefix(n.as_slice()) {
            return Some(NoFormatLine(n, false));
        }

        // replace trailing newline, if any, with space
        let CharRange {ch, next: i} = n.as_slice().char_range_at_reverse(n.len());
        if ch == '\n' {
            unsafe {
                let nmut = n.as_mut_bytes();
                nmut[i] = ' ' as u8;
            }
            if i > 0 {
                let CharRange {ch, next: _} = n.as_slice().char_range_at_reverse(i);
                if ch == '.' {
                    n.push_char(' ');
                }
            }
        }

        let nLen = n.len();
        // figure out the indent, prefix, and prefixindent ending points
        let (indEnd, pfxEnd, pfxIndEnd) = 
            if self.opts.use_prefix {
                let pfxEnd = poffset + self.opts.prefix.len();
                let nSlice = n.as_slice().slice_from(pfxEnd);
                let nSlice2 = nSlice.trim_left();
                (pfxEnd + nSlice.len() - nSlice2.len(), pfxEnd, poffset)
            } else {
                let nSlice = n.as_slice().trim_left();
                (nLen - nSlice.len(), 0, 0)
            };

        // indent length
        let indLen =
            if indEnd > 0 {
                self.displayed_length(n.as_slice().slice(pfxEnd, indEnd))
            } else {
                0
            };

        // prefix indent length
        let pfxIndLen =
            if pfxIndEnd > 0 {
                self.displayed_length(n.as_slice().slice_to(pfxIndEnd))
            } else {
                0
            };

        // if we are in uniform mode, all tabs after the indent should be replaced by spaces.
        // NOTE that in this implementation, [?!.]\t is NOT detected as a sentence break, but
        // [?!.]\t\t is. We could expand tabs to two spaces to force detection of tab as
        // sentence ending
        if self.opts.uniform {
            let tabinds: Vec<uint> = n.as_slice().slice_from(indEnd).char_indices().filter_map(|(i, c)| if c == '\t' { Some(i) } else { None }).collect();
            unsafe {
                let nmut = n.as_mut_bytes();
                for i in tabinds.iter() {
                    nmut[*i] = ' ' as u8;
                }
            }
        }

        Some(FormatLine(FileLine {
            line       : n,
            indent_end : indEnd,
            prefix_end : pfxEnd,
            pfxind_end : pfxIndEnd,
            indent_len : indLen,
            pfxind_len : pfxIndLen,
        }))
    }
}

// a paragraph : a collection of FileLines that are to be formatted
// plus info about the paragraph's indentation
// (but we only retain the String from the FileLine; the other info
// is only there to help us in deciding how to merge lines into Paragraphs
#[deriving(Show)]
pub struct Paragraph {
    lines           : Vec<String>,  // the lines of the file
    pub init_str    : String,       // string representing the init, that is, the first line's indent
    pub init_len    : uint,         // printable length of the init string considering TABWIDTH
    init_end        : uint,         // byte location of end of init in first line String
    pub indent_str  : String,       // string representing indent
    pub indent_len  : uint,         // length of above
    indent_end      : uint,         // byte location of end of indent (in crown and tagged mode, only applies to 2nd line and onward)
    pub pfxind_str  : String,       // string representing the prefix indent
    pub pfxind_len  : uint,         // length of above
    pub mail_header : bool          // we need to know if this is a mail header because we do word splitting differently in that case
}

// an iterator producing a stream of paragraphs from a stream of lines
// given a set of options.
// NOTE as you iterate through the paragraphs, any NoFormatLines are
// immediately dumped to stdout!
pub struct ParagraphStream<'a> {
    lines     : Peekable<Line,FileLines<'a>>,
    next_mail : bool,
    opts      : &'a FmtOptions,
}

impl<'a> ParagraphStream<'a> {
    pub fn new<'a>(opts: &'a FmtOptions, reader: &'a mut FileOrStdReader) -> ParagraphStream<'a> {
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
            let lSlice = line.line.as_slice();
            if lSlice.starts_with("From ") {
                true
            } else {
                let colonPosn =
                    match lSlice.find(':') {
                        Some(n) => n,
                        None => return false
                    };

                // header field must be nonzero length
                if colonPosn == 0 { return false; }

                return lSlice.slice_to(colonPosn).chars().all(|x| match x as uint {
                    y if y < 33 || y > 126 => false,
                    _ => true
                });
            }
        }
    }
}

impl<'a> Iterator<Result<Paragraph,String>> for ParagraphStream<'a> {
    fn next(&mut self) -> Option<Result<Paragraph,String>> {
        // return a NoFormatLine in an Err; it should immediately be output
        let noformat =
            match self.lines.peek() {
                None => return None,
                Some(l) => match l {
                    &FormatLine(_) => false,
                    &NoFormatLine(_, _) => true
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
        let mut pfxind_str = String::new();
        let mut pfxind_len = 0;
        let mut pLines = Vec::new();

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
                                &FormatLine(ref x) => x,
                                &NoFormatLine(..) => break
                            }
                        }
                    };

                if pLines.len() == 0 {
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
                            init_str.push_str(fl.line.as_slice().slice_to(fl.indent_end));
                            init_len = fl.indent_len + fl.pfxind_len + self.opts.prefix_len;
                            init_end = fl.indent_end;
                        } 

                        // these will be overwritten in the 2nd line of crown or tagged mode, but
                        // we are not guaranteed to get to the 2nd line, e.g., if the next line
                        // is a NoFormatLine or None. Thus, we set sane defaults the 1st time around
                        indent_str.push_str(fl.line.as_slice().slice(fl.prefix_end, fl.indent_end));
                        indent_len = fl.indent_len;
                        indent_end = fl.indent_end;

                        // in tagged mode, add 4 spaces of additional indenting by default
                        // (gnu fmt's behavior is different: it seems to find the closest column to
                        // indent_end that is divisible by 3. But honesly that behavior seems
                        // pretty arbitrary.
                        // Perhaps a better default would be 1 TABWIDTH? But ugh that's so big.
                        if self.opts.tagged {
                            indent_str.push_str("    ");
                            indent_len += 4;
                        }

                        if self.opts.use_prefix {
                            pfxind_str.push_str(fl.line.as_slice().slice_to(fl.pfxind_end));
                            pfxind_len = fl.pfxind_len;
                        }
                    }
                } else if in_mail {
                    // lines following mail headers must begin with spaces
                    if (self.opts.use_prefix && fl.pfxind_end == 0) || (!self.opts.use_prefix && fl.indent_end == 0) {
                        break;  // this line does not begin with spaces
                    }
                } else if !second_done && (self.opts.crown || self.opts.tagged) {
                    // now we have enough info to handle crown margin and tagged mode
                    if pfxind_len != fl.pfxind_len {
                        // in both crown and tagged modes we require that pfxind is the same
                        break;
                    } else if self.opts.tagged && (indent_end == fl.indent_end) {
                        // in tagged mode, indent also has to be different
                        break;
                    } else {
                        // this is part of the same paragraph, get the indent info from this line
                        indent_str.clear();
                        indent_str.push_str(fl.line.as_slice().slice(fl.prefix_end, fl.indent_end));
                        indent_len = fl.indent_len;
                        indent_end = fl.indent_end;
                    }
                    second_done = true;
                } else {
                    // detect mismatch
                    if (indent_end != fl.indent_end) || (indent_len != fl.indent_len) || (pfxind_len != fl.pfxind_len) {
                        break;
                    }
                }
            }

            pLines.push(self.lines.next().unwrap().get_fileline().line);

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
            lines       : pLines,
            init_str    : init_str,
            init_len    : init_len,
            init_end    : init_end,
            indent_str  : indent_str,
            indent_len  : indent_len,
            indent_end  : indent_end,
            pfxind_str  : pfxind_str,
            pfxind_len  : pfxind_len,
            mail_header : in_mail
        }))
    }
}

pub struct ParaWords<'a> {
    opts  : &'a FmtOptions,
    para  : &'a Paragraph,
    words : Vec<&'a str>
}

impl<'a> ParaWords<'a> {
    pub fn new<'a>(opts: &'a FmtOptions, para: &'a Paragraph) -> ParaWords<'a> {
        let mut pw = ParaWords { opts: opts, para: para, words: Vec::new() };
        pw.create_words();
        pw
    }

    fn create_words<'r>(&'r mut self) {
        if self.para.mail_header {
            // no extra spacing for mail headers; always exactly 1 space
            // safe to trim_left on every line of a mail header, since the
            // first line is guaranteed not to have any spaces
            self.words.push_all_move(self.para.lines.iter().flat_map(|x| x.as_slice().trim_left().words()).collect());
        } else {
            // first line
            self.words.push_all_move(
                if self.opts.crown || self.opts.tagged {
                    // crown and tagged mode has the "init" in the first line, so slice from there
                    WordSplit::new(self.opts.uniform, self.para.lines.get(0).as_slice().slice_from(self.para.init_end))
                } else {
                    // otherwise we slice from the indent
                    WordSplit::new(self.opts.uniform, self.para.lines.get(0).as_slice().slice_from(self.para.indent_end))
                }.collect());

            if self.para.lines.len() > 1 {
                let indent_end = self.para.indent_end;
                let uniform = self.opts.uniform;
                self.words.push_all_move(
                    self.para.lines.iter().skip(1)
                    .flat_map(|x| WordSplit::new(uniform, x.as_slice().slice_from(indent_end)))
                    .collect());
            }
        }
    }

    pub fn words(&'a self) -> Items<'a,&'a str> { return self.words.iter() }
}

struct WordSplit<'a> {
    uniform  : bool,
    string   : &'a str,
    length   : uint,
    position : uint
}

impl<'a> WordSplit<'a> {
    fn new<'a>(uniform: bool, string: &'a str) -> WordSplit<'a> {
        // wordsplits *must* start at a non-whitespace character
        let trim_string = string.trim_left();
        WordSplit { uniform: uniform, string: trim_string, length: string.len(), position: 0 }
    }

    fn is_punctuation(c: char) -> bool {
        match c {
            '!' | '.' | '?' => true,
            _ => false
        }
    }
}

impl<'a> Iterator<&'a str> for WordSplit<'a> {
    fn next(&mut self) -> Option<&'a str> {
        if self.position >= self.length {
            return None
        }

        let old_position = self.position;

        // find the start of the next whitespace segment
        let ws_start =
            match self.string.slice_from(old_position).find(|x: char| x.is_whitespace()) {
                None => self.length,
                Some(s) => s + old_position
            };

        if ws_start == self.length {
            self.position = self.length;
            return Some(self.string.slice_from(old_position));
        }

        // find the end of the next whitespace segment
        // note that this preserves the invariant that self.position points to
        // non-whitespace character OR end of string
        self.position =
            match self.string.slice_from(ws_start).find(|x: char| !x.is_whitespace()) {
                None => self.length,
                Some(s) => s + ws_start
            };

        let is_sentence_end = match self.string.char_range_at_reverse(ws_start) {
            CharRange { ch, next: _ } if WordSplit::is_punctuation(ch) => self.position - ws_start > 2,
            _ => false
        };

        Some(
            if self.uniform {
                // if the last non-whitespace character is a [?!.] and
                // there are two or more spaces, this is the end of a
                // sentence, so keep one extra space.
                if is_sentence_end {
                    self.string.slice(old_position, ws_start + 1)
                } else {
                    self.string.slice(old_position, ws_start)
                }
            } else {
                // in non-uniform mode, we just keep the whole thing
                // eventually we will want to annotate where the sentence boundaries are
                // so that we can give preference to splitting lines appropriately
                self.string.slice(old_position, self.position)
            }
        )
    }
}
