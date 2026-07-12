// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Rules to update the codebase using `rerast`

/// Converts try!() to ?
fn try_to_question_mark<T, E, X: From<E>>(r: Result<T, E>) -> Result<T, X> {
    replace!(try!(r) => r?);
    unreachable!()
}

fn trim_left_to_start(s: &str) {
    replace!(s.trim_left() => s.trim_start());
    replace!(s.trim_right() => s.trim_end());
}

fn trim_left_matches_to_start<P: FnMut(char) -> bool>(s: &str, inner: P) {
    replace!(s.trim_left_matches(inner) => s.trim_start_matches(inner));
    replace!(s.trim_right_matches(inner) => s.trim_end_matches(inner));
}
