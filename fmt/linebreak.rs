/*
 * This file is part of `fmt` from the uutils coreutils package.
 *
 * (c) kwantam <kwantam@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

// break_simple implements the "tight" breaking algorithm: print words until
// maxlength would be exceeded, then print a linebreak and indent and continue.
// Note that any first line indent should already have been printed before
// calling this function, and the length of said indent should be passed as
// init_len
pub fn break_simple<'a, T: Iterator<&'a str>>(s: &'a mut T, maxlen: uint, indent_str: &'a str, indent_len: uint, init_len: uint, uniform: bool, ostream: &mut Box<Writer>) -> uint {
    s.fold(init_len, |l, w| accum_words_simple(maxlen, indent_len, indent_str, ostream, uniform, l, w))
}

fn accum_words_simple(maxlen: uint, indent_len: uint, indent_str: &str, ostream: &mut Box<Writer>, uniform: bool, l: uint, w: &str) -> uint {
    let wlen = w.len();
    let lnew =
        if l + wlen > maxlen {
            silent_unwrap!(ostream.write("\n".as_bytes()));
            silent_unwrap!(ostream.write(indent_str.as_bytes()));
            indent_len
        } else {
            l
        };

    silent_unwrap!(ostream.write(w.as_bytes()));
    if uniform { silent_unwrap!(ostream.write(" ".as_bytes())); }
    lnew + wlen + 1
}
