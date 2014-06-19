/*
 * This file is part of `fmt` from the uutils coreutils package.
 *
 * (c) kwantam <kwantam@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use FmtOptions;
use parasplit::{Paragraph, ParaWords, WordInfo};

struct BreakArgs<'a> {
    opts       : &'a FmtOptions,
    init_len   : uint,
    indent_str : &'a str,
    indent_len : uint,
    uniform    : bool,
    ostream    : &'a mut Box<Writer>
}

impl<'a> BreakArgs<'a> {
    #[inline(always)]
    fn compute_width(&self, pre: uint, post: uint, posn: uint) -> uint {
        post + ((pre + posn) / self.opts.tabwidth + 1) * self.opts.tabwidth - posn
    }
}

pub fn break_lines(para: &Paragraph, opts: &FmtOptions, ostream: &mut Box<Writer>) {
    // indent
    let pIndent = para.indent_str.as_slice();
    let pIndentLen = para.indent_len;

    // words
    let pWords = ParaWords::new(opts, para);
    let mut pWords_words = pWords.words();

    // the first word will *always* appear on the first line
    // make sure of this here
    let (w, w_len) = match pWords_words.next() {
        Some(winfo) => (winfo.word, winfo.word_nchars),
        None => {
            silent_unwrap!(ostream.write_char('\n'));
            return;
        }
    };
    // print the init, if it exists, and get its length
    let pInitLen = w_len +
        if opts.crown || opts.tagged {
            // handle "init" portion
            silent_unwrap!(ostream.write(para.init_str.as_bytes()));
            para.init_len
        } else if !para.mail_header {
            // for non-(crown, tagged) that's the same as a normal indent
            silent_unwrap!(ostream.write(pIndent.as_bytes()));
            pIndentLen
        } else {
            // except that mail headers get no indent at all
            0
        };
    // write first word after writing init
    silent_unwrap!(ostream.write(w.as_bytes()));

    // does this paragraph require uniform spacing?
    let uniform = para.mail_header || opts.uniform;

    let mut break_args = BreakArgs {
        opts       : opts,
        init_len   : pInitLen,
        indent_str : pIndent,
        indent_len : pIndentLen,
        uniform    : uniform,
        ostream    : ostream
    };

    break_simple(&mut pWords_words, &mut break_args);
}

/*
 * break_simple implements the "tight" breaking algorithm: print words until
 * maxlength would be exceeded, then print a linebreak and indent and continue.
 * Note that any first line indent should already have been printed before
 * calling this function, and the displayed length of said indent passed as
 * args.init_len
 */
fn break_simple<'a,T: Iterator<&'a WordInfo<'a>>>(iter: &'a mut T, args: &mut BreakArgs<'a>) {
    iter.fold((args.init_len, false), |l, winfo| accum_words_simple(args, l, winfo));
    silent_unwrap!(args.ostream.write_char('\n'));
}

fn accum_words_simple<'a>(args: &mut BreakArgs<'a>, (l, prev_punct): (uint, bool), winfo: &'a WordInfo<'a>) -> (uint, bool) {
    // compute the length of this word, considering how tabs will expand at this position on the line
    let wlen = winfo.word_nchars +
        if winfo.before_tab.is_some() {
            args.compute_width(winfo.before_tab.unwrap(), winfo.after_tab, l)
        } else {
            winfo.after_tab
        };

    let splen =
        if args.uniform || winfo.new_line {
            if winfo.sentence_start || (winfo.new_line && prev_punct) { 2 }
            else { 1 }
        } else {
            0
        };

    if l + wlen + splen > args.opts.width {
        let wtrim = winfo.word.slice_from(winfo.word_start);
        silent_unwrap!(args.ostream.write_char('\n'));
        silent_unwrap!(args.ostream.write(args.indent_str.as_bytes()));
        silent_unwrap!(args.ostream.write(wtrim.as_bytes()));
        (args.indent_len + wtrim.len(), winfo.ends_punct)
    } else {
        if splen == 2 { silent_unwrap!(args.ostream.write("  ".as_bytes())); }
        else if splen == 1 { silent_unwrap!(args.ostream.write_char(' ')) }
        silent_unwrap!(args.ostream.write(winfo.word.as_bytes()));
        (l + wlen + splen, winfo.ends_punct)
    }
}

#[allow(dead_code)]
enum PreviousBreak<'a> {
    ParaStart,
    PrevBreak(&'a LineBreak<'a>)
}

#[allow(dead_code)]
struct LineBreak<'a> {
    prev       : PreviousBreak<'a>,
    breakafter : &'a str,
    demerits   : uint
}

// when comparing two LineBreaks, compare their demerits
#[allow(dead_code)]
impl<'a> PartialEq for LineBreak<'a> {
    fn eq(&self, other: &LineBreak) -> bool {
        self.demerits == other.demerits
    }
}

// NOTE "less than" in this case means "worse", i.e., more demerits
#[allow(dead_code)]
impl<'a> PartialOrd for LineBreak<'a> {
    fn lt(&self, other: &LineBreak) -> bool {
        self.demerits > other.demerits
    }
}

// we have to satisfy Eq to implement Ord
#[allow(dead_code)]
impl<'a> Eq for LineBreak<'a> {}

// NOTE again here we reverse the ordering:
// if other has more demerits, self is Greater
#[allow(dead_code)]
impl<'a> Ord for LineBreak<'a> {
    fn cmp(&self, other: &LineBreak) -> Ordering {
        other.demerits.cmp(&self.demerits)
    }
}

