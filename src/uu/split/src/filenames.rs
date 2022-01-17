//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
// spell-checker:ignore zaaa zaab zzaaaa zzzaaaaa
//! Compute filenames from a given index.
//!
//! The [`FilenameFactory`] can be used to convert a chunk index given
//! as a [`usize`] to a filename for that chunk.
//!
//! # Examples
//!
//! Create filenames of the form `chunk_??.txt`:
//!
//! ```rust,ignore
//! use crate::filenames::FilenameFactory;
//!
//! let prefix = "chunk_".to_string();
//! let suffix = ".txt".to_string();
//! let width = 2;
//! let use_numeric_suffix = false;
//! let factory = FilenameFactory::new(prefix, suffix, width, use_numeric_suffix);
//!
//! assert_eq!(factory.make(0).unwrap(), "chunk_aa.txt");
//! assert_eq!(factory.make(10).unwrap(), "chunk_ak.txt");
//! assert_eq!(factory.make(28).unwrap(), "chunk_bc.txt");
//! ```

/// Base 10 logarithm.
fn log10(n: usize) -> usize {
    (n as f64).log10() as usize
}

/// Base 26 logarithm.
fn log26(n: usize) -> usize {
    (n as f64).log(26.0) as usize
}

/// Convert a radix 10 number to a radix 26 number of the given width.
///
/// `n` is the radix 10 (that is, decimal) number to transform. This
/// function returns a [`Vec`] of unsigned integers representing the
/// digits, with the most significant digit first and the least
/// significant digit last. The returned `Vec` is always of length
/// `width`.
///
/// If the number `n` is too large to represent within `width` digits,
/// then this function returns `None`.
///
/// # Examples
///
/// ```rust,ignore
/// use crate::filenames::to_radix_26;
///
/// assert_eq!(to_radix_26(20, 2), Some(vec![0, 20]));
/// assert_eq!(to_radix_26(26, 2), Some(vec![1, 0]));
/// assert_eq!(to_radix_26(30, 2), Some(vec![1, 4]));
/// ```
fn to_radix_26(mut n: usize, width: usize) -> Option<Vec<u8>> {
    if width == 0 {
        return None;
    }
    // Use the division algorithm to repeatedly compute the quotient
    // and remainder of the number after division by the radix 26. The
    // successive quotients are the digits in radix 26, from most
    // significant to least significant.
    let mut result = vec![];
    for w in (0..width).rev() {
        let divisor = 26_usize.pow(w as u32);
        let (quotient, remainder) = (n / divisor, n % divisor);
        n = remainder;
        // If the quotient is equal to or greater than the radix, that
        // means the number `n` requires a greater width to be able to
        // represent it in radix 26.
        if quotient >= 26 {
            return None;
        }
        result.push(quotient as u8);
    }
    Some(result)
}

/// Convert a number between 0 and 25 into a lowercase ASCII character.
///
/// # Examples
///
/// ```rust,ignore
/// use crate::filenames::to_ascii_char;
///
/// assert_eq!(to_ascii_char(&0), Some('a'));
/// assert_eq!(to_ascii_char(&25), Some('z'));
/// assert_eq!(to_ascii_char(&26), None);
/// ```
fn to_ascii_char(n: &u8) -> Option<char> {
    // TODO In Rust v1.52.0 or later, use `char::from_digit`:
    // https://doc.rust-lang.org/std/primitive.char.html#method.from_digit
    //
    //     char::from_digit(*n as u32 + 10, 36)
    //
    // In that call, radix 36 is used because the characters in radix
    // 36 are [0-9a-z]. We want to exclude the the first ten of those
    // characters, so we add 10 to the number before conversion.
    //
    // Until that function is available, just add `n` to `b'a'` and
    // cast to `char`.
    if *n < 26 {
        Some((b'a' + n) as char)
    } else {
        None
    }
}

/// Fixed width alphabetic string representation of index `i`.
///
/// If `i` is greater than or equal to the number of lowercase ASCII
/// strings that can be represented in the given `width`, then this
/// function returns `None`.
///
/// # Examples
///
/// ```rust,ignore
/// use crate::filenames::str_prefix_fixed_width;
///
/// assert_eq!(str_prefix_fixed_width(0, 2).as_deref(), "aa");
/// assert_eq!(str_prefix_fixed_width(675, 2).as_deref(), "zz");
/// assert_eq!(str_prefix_fixed_width(676, 2), None);
/// ```
fn str_prefix_fixed_width(i: usize, width: usize) -> Option<String> {
    to_radix_26(i, width)?.iter().map(to_ascii_char).collect()
}

/// Dynamically sized alphabetic string representation of index `i`.
///
/// The size of the returned string starts at two then grows by 2 if
/// `i` is sufficiently large.
///
/// # Examples
///
/// ```rust,ignore
/// use crate::filenames::str_prefix;
///
/// assert_eq!(str_prefix(0), "aa");
/// assert_eq!(str_prefix(649), "yz");
/// assert_eq!(str_prefix(650), "zaaa");
/// assert_eq!(str_prefix(651), "zaab");
/// ```
fn str_prefix(i: usize) -> Option<String> {
    // This number tells us the order of magnitude of `i`, with a
    // slight adjustment.
    //
    // We shift by 26 so that
    //
    // * if `i` is in the interval [0, 26^2 - 26), then `d` is 1,
    // * if `i` is in the interval [26^2 - 26, 26^3 - 26), then `d` is 2,
    // * if `i` is in the interval [26^3 - 26, 26^4 - 26), then `d` is 3,
    //
    // and so on. This will allow us to compute how many leading "z"
    // characters need to appear in the string and how many characters
    // to format to the right of those.
    let d = log26(i + 26);

    // This is the number of leading "z" characters.
    //
    // For values of `i` less than 26^2 - 26, the returned string is
    // just the radix 26 representation of that number with a width of
    // two (using the lowercase ASCII characters as the digits).
    //
    // * if `i` is 26^2 - 26, then the returned string is "zaa",
    // * if `i` is 26^3 - 26, then the returned string is "zzaaaa",
    // * if `i` is 26^4 - 26, then the returned string is "zzzaaaaa",
    //
    // and so on. As you can see, the number of leading "z"s there is
    // linearly increasing by 1 for each order of magnitude.
    let num_fill_chars = d - 1;

    // This is the number of characters after the leading "z" characters.
    let width = d + 1;

    // This is the radix 10 number to render in radix 26, to the right
    // of the leading "z"s.
    let number = (i + 26) - 26_usize.pow(d as u32);

    // This is the radix 26 number to render after the leading "z"s,
    // collected in a `String`.
    //
    // For example, if `i` is 789, then `number` is 789 + 26 - 676,
    // which equals 139. In radix 26 and assuming a `width` of 3, this
    // number is
    //
    //     [0, 5, 9]
    //
    // with the most significant digit on the left and the least
    // significant digit on the right. After translating to ASCII
    // lowercase letters, this becomes "afj".
    let digits = str_prefix_fixed_width(number, width)?;

    // `empty` is just the empty string, to be displayed with a width
    // of `num_fill_chars` and with blank spaces filled with the
    // character "z".
    //
    // `digits` is as described in the previous comment.
    Some(format!(
        "{empty:z<num_fill_chars$}{digits}",
        empty = "",
        num_fill_chars = num_fill_chars,
        digits = digits
    ))
}

/// Fixed width numeric string representation of index `i`.
///
/// If `i` is greater than or equal to the number of numbers that can
/// be represented in the given `width`, then this function returns
/// `None`.
///
/// # Examples
///
/// ```rust,ignore
/// use crate::filenames::num_prefix_fixed_width;
///
/// assert_eq!(num_prefix_fixed_width(0, 2).as_deref(), "89");
/// assert_eq!(num_prefix_fixed_width(99, 2).as_deref(), "9000");
/// assert_eq!(num_prefix_fixed_width(100, 2), None);
/// ```
fn num_prefix_fixed_width(i: usize, width: usize) -> Option<String> {
    let max = 10_usize.pow(width as u32);
    if i >= max {
        None
    } else {
        Some(format!("{i:0width$}", i = i, width = width))
    }
}

/// Dynamically sized numeric string representation of index `i`.
///
/// The size of the returned string starts at two then grows by 2 if
/// `i` is sufficiently large.
///
/// # Examples
///
/// ```rust,ignore
/// use crate::filenames::num_prefix;
///
/// assert_eq!(num_prefix(89), "89");
/// assert_eq!(num_prefix(90), "9000");
/// assert_eq!(num_prefix(91), "9001");
/// ```
fn num_prefix(i: usize) -> String {
    // This number tells us the order of magnitude of `i`, with a
    // slight adjustment.
    //
    // We shift by 10 so that
    //
    // * if `i` is in the interval [0, 90), then `d` is 1,
    // * if `i` is in the interval [90, 990), then `d` is 2,
    // * if `i` is in the interval [990, 9990), then `d` is 3,
    //
    // and so on. This will allow us to compute how many leading "9"
    // characters need to appear in the string and how many digits to
    // format to the right of those.
    let d = log10(i + 10);

    // This is the number of leading "9" characters.
    //
    // For values of `i` less than 90, the returned string is just
    // that number padded by a 0 to ensure the width is 2, but
    //
    // * if `i` is 90, then the returned string is "900",
    // * if `i` is 990, then the returned string is "990000",
    // * if `i` is 9990, then the returned string is "99900000",
    //
    // and so on. As you can see, the number of leading 9s there is
    // linearly increasing by 1 for each order of magnitude.
    let num_fill_chars = d - 1;

    // This is the number of characters after the leading "9" characters.
    let width = d + 1;

    // This is the number to render after the leading "9"s.
    //
    // For example, if `i` is 5732, then the returned string is
    // "994742". After the two "9" characters is the number 4742,
    // which equals 5732 + 10 - 1000.
    let number = (i + 10) - 10_usize.pow(d as u32);

    // `empty` is just the empty string, to be displayed with a width
    // of `num_fill_chars` and with blank spaces filled with the
    // character "9".
    //
    // `number` is the next remaining part of the number to render;
    // for small numbers we pad with 0 and enforce a minimum width.
    format!(
        "{empty:9<num_fill_chars$}{number:0width$}",
        empty = "",
        num_fill_chars = num_fill_chars,
        number = number,
        width = width
    )
}

/// Compute filenames from a given index.
///
/// The [`FilenameFactory`] can be used to convert a chunk index given
/// as a [`usize`] to a filename for that chunk.
///
/// The general form of filenames produced by instances of this struct is
///
/// ```ignore
/// {prefix}{suffix}{additional_suffix}
/// ```
///
/// If `suffix_length` is a positive integer, then the `suffix`
/// portion will be of exactly that length. If `suffix_length` is 0,
/// then the length of the `suffix` portion will grow dynamically to
/// accommodate any chunk index. In that case, the length begins at 2
/// and increases by 2 when the chunk index becomes sufficiently
/// large.
///
/// If `use_numeric_suffix` is `true`, then the `suffix` portion will
/// be nonnegative integers. If `false`, then the `suffix` will
/// comprise lowercase ASCII characters.
///
/// # Examples
///
/// Create filenames of the form `chunk_??.txt`:
///
/// ```rust,ignore
/// use crate::filenames::FilenameFactory;
///
/// let prefix = "chunk_".to_string();
/// let suffix = ".txt".to_string();
/// let width = 2;
/// let use_numeric_suffix = false;
/// let factory = FilenameFactory::new(prefix, suffix, width, use_numeric_suffix);
///
/// assert_eq!(factory.make(0).unwrap(), "chunk_aa.txt");
/// assert_eq!(factory.make(10).unwrap(), "chunk_ak.txt");
/// assert_eq!(factory.make(28).unwrap(), "chunk_bc.txt");
/// ```
///
/// Set `suffix_length` to 0 for filename sizes that grow dynamically:
///
/// ```rust,ignore
/// use crate::filenames::FilenameFactory;
///
/// let prefix = String::new();
/// let suffix = String::new();
/// let width = 0;
/// let use_numeric_suffix = false;
/// let factory = FilenameFactory::new(prefix, suffix, width, use_numeric_suffix);
///
/// assert_eq!(factory.make(0).unwrap(), "aa");
/// assert_eq!(factory.make(1).unwrap(), "ab");
/// assert_eq!(factory.make(649).unwrap(), "yz");
/// assert_eq!(factory.make(650).unwrap(), "zaaa");
/// assert_eq!(factory.make(6551).unwrap(), "zaab");
/// ```
pub struct FilenameFactory {
    additional_suffix: String,
    prefix: String,
    suffix_length: usize,
    use_numeric_suffix: bool,
}

impl FilenameFactory {
    /// Create a new instance of this struct.
    ///
    /// For an explanation of the parameters, see the struct documentation.
    pub fn new(
        prefix: String,
        additional_suffix: String,
        suffix_length: usize,
        use_numeric_suffix: bool,
    ) -> FilenameFactory {
        FilenameFactory {
            prefix,
            additional_suffix,
            suffix_length,
            use_numeric_suffix,
        }
    }

    /// Construct the filename for the specified element of the output collection of files.
    ///
    /// For an explanation of the parameters, see the struct documentation.
    ///
    /// If `suffix_length` has been set to a positive integer and `i`
    /// is greater than or equal to the number of strings that can be
    /// represented within that length, then this returns `None`. For
    /// example:
    ///
    /// ```rust,ignore
    /// use crate::filenames::FilenameFactory;
    ///
    /// let prefix = String::new();
    /// let suffix = String::new();
    /// let width = 1;
    /// let use_numeric_suffix = true;
    /// let factory = FilenameFactory::new(prefix, suffix, width, use_numeric_suffix);
    ///
    /// assert_eq!(factory.make(10), None);
    /// ```
    pub fn make(&self, i: usize) -> Option<String> {
        let prefix = self.prefix.clone();
        let suffix1 = match (self.use_numeric_suffix, self.suffix_length) {
            (true, 0) => Some(num_prefix(i)),
            (false, 0) => str_prefix(i),
            (true, width) => num_prefix_fixed_width(i, width),
            (false, width) => str_prefix_fixed_width(i, width),
        }?;
        let suffix2 = &self.additional_suffix;
        Some(prefix + &suffix1 + suffix2)
    }
}

#[cfg(test)]
mod tests {
    use crate::filenames::num_prefix;
    use crate::filenames::num_prefix_fixed_width;
    use crate::filenames::str_prefix;
    use crate::filenames::str_prefix_fixed_width;
    use crate::filenames::to_ascii_char;
    use crate::filenames::to_radix_26;
    use crate::filenames::FilenameFactory;

    #[test]
    fn test_to_ascii_char() {
        assert_eq!(to_ascii_char(&0), Some('a'));
        assert_eq!(to_ascii_char(&5), Some('f'));
        assert_eq!(to_ascii_char(&25), Some('z'));
        assert_eq!(to_ascii_char(&26), None);
    }

    #[test]
    fn test_to_radix_26_exceed_width() {
        assert_eq!(to_radix_26(1, 0), None);
        assert_eq!(to_radix_26(26, 1), None);
        assert_eq!(to_radix_26(26 * 26, 2), None);
    }

    #[test]
    fn test_to_radix_26_width_one() {
        assert_eq!(to_radix_26(0, 1), Some(vec![0]));
        assert_eq!(to_radix_26(10, 1), Some(vec![10]));
        assert_eq!(to_radix_26(20, 1), Some(vec![20]));
        assert_eq!(to_radix_26(25, 1), Some(vec![25]));
    }

    #[test]
    fn test_to_radix_26_width_two() {
        assert_eq!(to_radix_26(0, 2), Some(vec![0, 0]));
        assert_eq!(to_radix_26(10, 2), Some(vec![0, 10]));
        assert_eq!(to_radix_26(20, 2), Some(vec![0, 20]));
        assert_eq!(to_radix_26(25, 2), Some(vec![0, 25]));

        assert_eq!(to_radix_26(26, 2), Some(vec![1, 0]));
        assert_eq!(to_radix_26(30, 2), Some(vec![1, 4]));

        assert_eq!(to_radix_26(26 * 2, 2), Some(vec![2, 0]));
        assert_eq!(to_radix_26(26 * 26 - 1, 2), Some(vec![25, 25]));
    }

    #[test]
    fn test_str_prefix_dynamic_width() {
        assert_eq!(str_prefix(0).as_deref(), Some("aa"));
        assert_eq!(str_prefix(1).as_deref(), Some("ab"));
        assert_eq!(str_prefix(2).as_deref(), Some("ac"));
        assert_eq!(str_prefix(25).as_deref(), Some("az"));

        assert_eq!(str_prefix(26).as_deref(), Some("ba"));
        assert_eq!(str_prefix(27).as_deref(), Some("bb"));
        assert_eq!(str_prefix(28).as_deref(), Some("bc"));
        assert_eq!(str_prefix(51).as_deref(), Some("bz"));

        assert_eq!(str_prefix(52).as_deref(), Some("ca"));

        assert_eq!(str_prefix(26 * 25 - 1).as_deref(), Some("yz"));
        assert_eq!(str_prefix(26 * 25).as_deref(), Some("zaaa"));
        assert_eq!(str_prefix(26 * 25 + 1).as_deref(), Some("zaab"));
    }

    #[test]
    fn test_num_prefix_dynamic_width() {
        assert_eq!(num_prefix(0), "00");
        assert_eq!(num_prefix(9), "09");
        assert_eq!(num_prefix(17), "17");
        assert_eq!(num_prefix(89), "89");
        assert_eq!(num_prefix(90), "9000");
        assert_eq!(num_prefix(91), "9001");
        assert_eq!(num_prefix(989), "9899");
        assert_eq!(num_prefix(990), "990000");
    }

    #[test]
    fn test_str_prefix_fixed_width() {
        assert_eq!(str_prefix_fixed_width(0, 2).as_deref(), Some("aa"));
        assert_eq!(str_prefix_fixed_width(1, 2).as_deref(), Some("ab"));
        assert_eq!(str_prefix_fixed_width(26, 2).as_deref(), Some("ba"));
        assert_eq!(
            str_prefix_fixed_width(26 * 26 - 1, 2).as_deref(),
            Some("zz")
        );
        assert_eq!(str_prefix_fixed_width(26 * 26, 2).as_deref(), None);
    }

    #[test]
    fn test_num_prefix_fixed_width() {
        assert_eq!(num_prefix_fixed_width(0, 2).as_deref(), Some("00"));
        assert_eq!(num_prefix_fixed_width(1, 2).as_deref(), Some("01"));
        assert_eq!(num_prefix_fixed_width(99, 2).as_deref(), Some("99"));
        assert_eq!(num_prefix_fixed_width(100, 2).as_deref(), None);
    }

    #[test]
    fn test_alphabetic_suffix() {
        let factory = FilenameFactory::new("123".to_string(), "789".to_string(), 3, false);
        assert_eq!(factory.make(0).unwrap(), "123aaa789");
        assert_eq!(factory.make(1).unwrap(), "123aab789");
        assert_eq!(factory.make(28).unwrap(), "123abc789");
    }

    #[test]
    fn test_numeric_suffix() {
        let factory = FilenameFactory::new("abc".to_string(), "xyz".to_string(), 3, true);
        assert_eq!(factory.make(0).unwrap(), "abc000xyz");
        assert_eq!(factory.make(1).unwrap(), "abc001xyz");
        assert_eq!(factory.make(123).unwrap(), "abc123xyz");
    }
}
