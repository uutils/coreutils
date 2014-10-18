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
use std::i64;
use std::cmp;
use std::mem;
use std::num;

struct BreakArgs<'a> {
    opts       : &'a FmtOptions,
    init_len   : uint,
    indent_str : &'a str,
    indent_len : uint,
    uniform    : bool,
    ostream    : &'a mut Box<Writer+'static>
}

impl<'a> BreakArgs<'a> {
    #[inline(always)]
    fn compute_width<'b>(&self, winfo: &WordInfo<'b>, posn: uint, fresh: bool) -> uint {
        if fresh {
            0
        } else {
            let post = winfo.after_tab;
            match winfo.before_tab {
                None => post,
                Some(pre) => post + ((pre + posn) / self.opts.tabwidth + 1) * self.opts.tabwidth - posn
            }
        }
    }
}

pub fn break_lines(para: &Paragraph, opts: &FmtOptions, ostream: &mut Box<Writer+'static>) {
    // indent
    let p_indent = para.indent_str.as_slice();
    let p_indent_len = para.indent_len;

    // words
    let p_words = ParaWords::new(opts, para);
    let mut p_words_words = p_words.words();

    // the first word will *always* appear on the first line
    // make sure of this here
    let (w, w_len) = match p_words_words.next() {
        Some(winfo) => (winfo.word, winfo.word_nchars),
        None => {
            silent_unwrap!(ostream.write_char('\n'));
            return;
        }
    };
    // print the init, if it exists, and get its length
    let p_init_len = w_len +
        if opts.crown || opts.tagged {
            // handle "init" portion
            silent_unwrap!(ostream.write(para.init_str.as_bytes()));
            para.init_len
        } else if !para.mail_header {
            // for non-(crown, tagged) that's the same as a normal indent
            silent_unwrap!(ostream.write(p_indent.as_bytes()));
            p_indent_len
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
        init_len   : p_init_len,
        indent_str : p_indent,
        indent_len : p_indent_len,
        uniform    : uniform,
        ostream    : ostream
    };

    if opts.quick || para.mail_header {
        break_simple(p_words_words, &mut break_args);
    } else {
        break_knuth_plass(p_words_words, &mut break_args);
    }
}

// break_simple implements a "greedy" breaking algorithm: print words until
// maxlength would be exceeded, then print a linebreak and indent and continue.
fn break_simple<'a, T: Iterator<&'a WordInfo<'a>>>(mut iter: T, args: &mut BreakArgs<'a>) {
    iter.fold((args.init_len, false), |l, winfo| accum_words_simple(args, l, winfo));
    silent_unwrap!(args.ostream.write_char('\n'));
}

#[inline(always)]
fn accum_words_simple<'a>(args: &mut BreakArgs<'a>, (l, prev_punct): (uint, bool), winfo: &'a WordInfo<'a>) -> (uint, bool) {
    // compute the length of this word, considering how tabs will expand at this position on the line
    let wlen = winfo.word_nchars + args.compute_width(winfo, l, false);

    let slen = compute_slen(args.uniform, winfo.new_line, winfo.sentence_start, prev_punct);

    if l + wlen + slen > args.opts.width {
        write_newline(args.indent_str, args.ostream);
        write_with_spaces(winfo.word.slice_from(winfo.word_start), 0, args.ostream);
        (args.indent_len + winfo.word_nchars, winfo.ends_punct)
    } else {
        write_with_spaces(winfo.word, slen, args.ostream);
        (l + wlen + slen, winfo.ends_punct)
    }
}

// break_knuth_plass implements an "optimal" breaking algorithm in the style of
//    Knuth, D.E., and Plass, M.F. "Breaking Paragraphs into Lines." in Software,
//    Practice and Experience. Vol. 11, No. 11, November 1981.
//    http://onlinelibrary.wiley.com/doi/10.1002/spe.4380111102/pdf
fn break_knuth_plass<'a, T: Clone + Iterator<&'a WordInfo<'a>>>(mut iter: T, args: &mut BreakArgs<'a>) {
    // run the algorithm to get the breakpoints
    let breakpoints = find_kp_breakpoints(iter.clone(), args);

    // iterate through the breakpoints (note that breakpoints is in reverse break order, so we .rev() it
    let (mut prev_punct, mut fresh) =
        breakpoints.iter().rev().fold((false, false), |(mut prev_punct, mut fresh), &(next_break, break_before)| {
            if fresh {
                write_newline(args.indent_str, args.ostream);
            }
            // at each breakpoint, keep emitting words until we find the word matching this breakpoint
            for winfo in iter {
                let (slen, word) = slice_if_fresh(fresh, winfo.word, winfo.word_start, args.uniform,
                                                  winfo.new_line, winfo.sentence_start, prev_punct);
                fresh = false;
                prev_punct = winfo.ends_punct;

                // We find identical breakpoints here by comparing addresses of the references.
                // This is OK because the backing vector is not mutating once we are linebreaking.
                let winfo_ptr = winfo as *const _;
                let next_break_ptr = next_break as *const _;
                if winfo_ptr == next_break_ptr {
                    // OK, we found the matching word
                    if break_before {
                        write_newline(args.indent_str, args.ostream);
                        write_with_spaces(winfo.word.slice_from(winfo.word_start), 0, args.ostream);
                    } else {
                        // breaking after this word, so that means "fresh" is true for the next iteration
                        write_with_spaces(word, slen, args.ostream);
                        fresh = true;
                    }
                    break;
                } else {
                    write_with_spaces(word, slen, args.ostream);
                }
            }
            (prev_punct, fresh)
        });

    // after the last linebreak, write out the rest of the final line.
    for winfo in iter {
        if fresh {
            write_newline(args.indent_str, args.ostream);
        }
        let (slen, word) = slice_if_fresh(fresh, winfo.word, winfo.word_start, args.uniform,
                                          winfo.new_line, winfo.sentence_start, prev_punct);
        prev_punct = winfo.ends_punct;
        fresh = false;
        write_with_spaces(word, slen, args.ostream);
    }
    silent_unwrap!(args.ostream.write_char('\n'));
}

struct LineBreak<'a> {
    prev         : uint,
    linebreak    : Option<&'a WordInfo<'a>>,
    break_before : bool,
    demerits     : i64,
    prev_rat     : f32,
    length       : uint,
    fresh        : bool
}

fn find_kp_breakpoints<'a, T: Iterator<&'a WordInfo<'a>>>(iter: T, args: &BreakArgs<'a>) -> Vec<(&'a WordInfo<'a>, bool)> {
    let mut iter = iter.peekable();
    // set up the initial null linebreak
    let mut linebreaks = vec!(LineBreak {
        prev         : 0,
        linebreak    : None,
        break_before : false,
        demerits     : 0,
        prev_rat     : 0.0f32,
        length       : args.init_len,
        fresh        : false
    });
    // this vec holds the current active linebreaks; next_ holds the breaks that will be active for the next word
    let active_breaks = &mut vec!(0);
    let next_active_breaks = &mut vec!();

    let stretch = (args.opts.width - args.opts.goal) as int;
    let minlength = args.opts.goal - stretch as uint;
    let mut new_linebreaks = vec!();
    let mut is_sentence_start = false;
    let mut least_demerits = 0;
    loop {
        let w =
            match iter.next() {
                None => break,
                Some(w) => w
            };

        // if this is the last word, we don't add additional demerits for this break
        let (is_last_word, is_sentence_end) =
            match iter.peek() {
                None => (true, true),
                Some(&&WordInfo { sentence_start: st, new_line: nl, .. }) => (false, st || (nl && w.ends_punct))
            };

        // should we be adding extra space at the beginning of the next sentence?
        let slen = compute_slen(args.uniform, w.new_line, is_sentence_start, false);

        let mut ld_new = i64::MAX;
        let mut ld_next = i64::MAX;
        let mut ld_idx = 0;
        new_linebreaks.clear();
        next_active_breaks.clear();
        // go through each active break, extending it and possibly adding a new active
        // break if we are above the minimum required length
        for &i in active_breaks.iter() {
            let active = linebreaks.get_mut(i);
            // normalize demerits to avoid overflow, and record if this is the least
            active.demerits -= least_demerits;
            if active.demerits < ld_next {
                ld_next = active.demerits;
                ld_idx = i;
            }

            // get the new length
            let tlen = w.word_nchars + args.compute_width(w, active.length, active.fresh) + slen + active.length;

            // if tlen is longer than args.opts.width, we drop this break from the active list
            // otherwise, we extend the break, and possibly add a new break at this point
            if tlen <= args.opts.width {
                // this break will still be active next time
                next_active_breaks.push(i);
                // we can put this word on this line
                active.fresh = false;
                active.length = tlen;

                // if we're above the minlength, we can also consider breaking here
                if tlen >= minlength {
                    let (new_demerits, new_ratio) =
                        if is_last_word {
                            // there is no penalty for the final line's length
                            (0, 0.0)
                        } else {
                            compute_demerits((args.opts.goal - tlen) as int, stretch, w.word_nchars as int, active.prev_rat)
                        };

                    // do not even consider adding a line that has too many demerits
                    // also, try to detect overflow by checking signum
                    let total_demerits = new_demerits + active.demerits;
                    if new_demerits < BAD_INFTY_SQ && total_demerits < ld_new && num::signum(active.demerits) <= num::signum(new_demerits) {
                        ld_new = total_demerits;
                        new_linebreaks.push(LineBreak {
                            prev         : i,
                            linebreak    : Some(w),
                            break_before : false,
                            demerits     : total_demerits,
                            prev_rat     : new_ratio,
                            length       : args.indent_len,
                            fresh        : true
                        });
                    }
                }
            }
        }

        // if we generated any new linebreaks, add the last one to the list
        // the last one is always the best because we don't add to new_linebreaks unless
        // it's better than the best one so far
        match new_linebreaks.pop() {
            None => (),
            Some(lb) => {
                next_active_breaks.push(linebreaks.len());
                linebreaks.push(lb);
            }
        }

        if next_active_breaks.is_empty() {
            // every potential linebreak is too long! choose the linebreak with the least demerits, ld_idx
            let new_break = restart_active_breaks(args, &linebreaks[ld_idx], ld_idx, w, slen, minlength);
            next_active_breaks.push(linebreaks.len());
            linebreaks.push(new_break);
            least_demerits = 0;
        } else {
            // next time around, normalize out the demerits fields
            // on active linebreaks to make overflow less likely
            least_demerits = cmp::max(ld_next, 0);
        }
        // swap in new list of active breaks
        mem::swap(active_breaks, next_active_breaks);
        // If this was the last word in a sentence, the next one must be the first in the next.
        is_sentence_start = is_sentence_end;
    }

    // return the best path
    build_best_path(&linebreaks, active_breaks)
}

#[inline(always)]
fn build_best_path<'a>(paths: &Vec<LineBreak<'a>>, active: &Vec<uint>) -> Vec<(&'a WordInfo<'a>, bool)> {
    let mut breakwords = vec!();
    // of the active paths, we select the one with the fewest demerits
    let mut best_idx = match active.iter().min_by(|&&a| paths[a].demerits) {
        None => crash!(1, "Failed to find a k-p linebreak solution. This should never happen."),
        Some(&s) => s
    };

    // now, chase the pointers back through the break list, recording
    // the words at which we should break
    loop {
        let next_best = paths[best_idx];
        match next_best.linebreak {
            None => return breakwords,
            Some(prev) => {
                breakwords.push((prev, next_best.break_before));
                best_idx = next_best.prev
            }
        }
    }
}

// "infinite" badness is more like (1+BAD_INFTY)^2 because of how demerits are computed
const BAD_INFTY: i64 = 10000000;
const BAD_INFTY_SQ: i64 = BAD_INFTY * BAD_INFTY;
// badness = BAD_MULT * abs(r) ^ 3
const BAD_MULT: f32 = 100.0;
// DR_MULT is multiplier for delta-R between lines
const DR_MULT: f32 = 600.0;
// DL_MULT is penalty multiplier for short words at end of line
const DL_MULT: f32 = 300.0;

#[inline(always)]
fn compute_demerits(delta_len: int, stretch: int, wlen: int, prev_rat: f32) -> (i64, f32) {
    // how much stretch are we using?
    let ratio =
        if delta_len == 0 {
            0.0f32
        } else {
            delta_len as f32 / stretch as f32
        };

    // compute badness given the stretch ratio
    let bad_linelen =
        if num::abs(ratio) > 1.0f32 {
            BAD_INFTY
        } else {
            (BAD_MULT * num::abs(num::pow(ratio, 3))) as i64
        };

    // we penalize lines ending in really short words
    let bad_wordlen =
        if wlen >= stretch {
            0
        } else {
            (DL_MULT * num::abs(num::pow((stretch - wlen) as f32 / (stretch - 1) as f32, 3))) as i64
        };

    // we penalize lines that have very different ratios from previous lines
    let bad_delta_r = (DR_MULT * num::abs(num::pow((ratio - prev_rat) / 2.0, 3))) as i64;

    let demerits = num::pow(1 + bad_linelen + bad_wordlen + bad_delta_r, 2);

    (demerits, ratio)
}

#[inline(always)]
fn restart_active_breaks<'a>(args: &BreakArgs<'a>, active: &LineBreak<'a>, act_idx: uint, w: &'a WordInfo<'a>, slen: uint, min: uint) -> LineBreak<'a> {
    let (break_before, line_length) =
        if active.fresh {
            // never break before a word if that word would be the first on a line
            (false, args.indent_len)
        } else {
            // choose the lesser evil: breaking too early, or breaking too late
            let wlen = w.word_nchars + args.compute_width(w, active.length, active.fresh);
            let underlen: int = (min - active.length) as int;
            let overlen: int = ((wlen + slen + active.length) - args.opts.width) as int;
            if overlen > underlen {
                // break early, put this word on the next line
                (true, args.indent_len + w.word_nchars)
            } else {
                (false, args.indent_len)
            }
        };

    // restart the linebreak. This will be our only active path.
    LineBreak {
        prev         : act_idx,
        linebreak    : Some(w),
        break_before : break_before,
        demerits     : 0, // this is the only active break, so we can reset the demerit count
        prev_rat     : if break_before { 1.0 } else { -1.0 },
        length       : line_length,
        fresh        : !break_before
    }
}

// Number of spaces to add before a word, based on mode, newline, sentence start.
#[inline(always)]
fn compute_slen(uniform: bool, newline: bool, start: bool, punct: bool) -> uint {
    if uniform || newline {
        if start || (newline && punct) {
            2
        } else {
            1
        }
    } else {
        0
    }
}

// If we're on a fresh line, slen=0 and we slice off leading whitespace.
// Otherwise, compute slen and leave whitespace alone.
#[inline(always)]
fn slice_if_fresh<'a>(fresh: bool, word: &'a str, start: uint, uniform: bool, newline: bool, sstart: bool, punct: bool) -> (uint, &'a str) {
    if fresh {
        (0, word.slice_from(start))
    } else {
        (compute_slen(uniform, newline, sstart, punct), word)
    }
}

// Write a newline and add the indent.
#[inline(always)]
fn write_newline(indent: &str, ostream: &mut Box<Writer>) {
    silent_unwrap!(ostream.write_char('\n'));
    silent_unwrap!(ostream.write(indent.as_bytes()));
}

// Write the word, along with slen spaces.
#[inline(always)]
fn write_with_spaces(word: &str, slen: uint, ostream: &mut Box<Writer>) {
    if slen == 2 {
        silent_unwrap!(ostream.write("  ".as_bytes()));
    } else if slen == 1 {
        silent_unwrap!(ostream.write_char(' '));
    }
    silent_unwrap!(ostream.write(word.as_bytes()));
}
