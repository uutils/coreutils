// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Compare two version strings.

use std::cmp::Ordering;

/// Compares the non-digit parts of a version.
/// Special cases: ~ are before everything else, even ends ("a~" < "a")
/// Letters are before non-letters
fn version_non_digit_cmp(a: &[u8], b: &[u8]) -> Ordering {
    let mut a_chars = a.iter();
    let mut b_chars = b.iter();
    loop {
        match (a_chars.next(), b_chars.next()) {
            (Some(c1), Some(c2)) if c1 == c2 => {}
            (None, None) => return Ordering::Equal,
            (_, Some(b'~')) => return Ordering::Greater,
            (Some(b'~'), _) => return Ordering::Less,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(c1), Some(c2)) if c1.is_ascii_alphabetic() && !c2.is_ascii_alphabetic() => {
                return Ordering::Less;
            }
            (Some(c1), Some(c2)) if !c1.is_ascii_alphabetic() && c2.is_ascii_alphabetic() => {
                return Ordering::Greater;
            }
            (Some(c1), Some(c2)) => return c1.cmp(c2),
        }
    }
}

/// Remove file endings matching the regex `(\.[A-Za-z~][A-Za-z0-9~]*)*$`
fn remove_file_ending(a: &[u8]) -> &[u8] {
    let mut ending_start = None;
    let mut prev_was_dot = false;
    for (idx, &char) in a.iter().enumerate() {
        if char == b'.' {
            if ending_start.is_none() || prev_was_dot {
                ending_start = Some(idx);
            }
            prev_was_dot = true;
        } else if prev_was_dot {
            prev_was_dot = false;
            if !char.is_ascii_alphabetic() && char != b'~' {
                ending_start = None;
            }
        } else if !char.is_ascii_alphanumeric() && char != b'~' {
            ending_start = None;
        }
    }
    if prev_was_dot {
        ending_start = None;
    }
    if let Some(ending_start) = ending_start {
        &a[..ending_start]
    } else {
        a
    }
}

/// Compare two version strings.
pub fn version_cmp(mut a: &[u8], mut b: &[u8]) -> Ordering {
    let str_cmp = a.cmp(b);
    if str_cmp == Ordering::Equal {
        return str_cmp;
    }

    // Special cases:
    // 1. Empty strings
    match (a.is_empty(), b.is_empty()) {
        (true, false) => return Ordering::Less,
        (false, true) => return Ordering::Greater,
        (true, true) => unreachable!(),
        (false, false) => {}
    }
    // 2. Dots
    match (a == b".", b == b".") {
        (true, false) => return Ordering::Less,
        (false, true) => return Ordering::Greater,
        (true, true) => unreachable!(),
        (false, false) => {}
    }
    // 3. Two Dots
    match (a == b"..", b == b"..") {
        (true, false) => return Ordering::Less,
        (false, true) => return Ordering::Greater,
        (true, true) => unreachable!(),
        (false, false) => {}
    }
    // 4. Strings starting with a dot
    match (a.starts_with(b"."), b.starts_with(b".")) {
        (true, false) => return Ordering::Less,
        (false, true) => return Ordering::Greater,
        (true, true) => {
            // Strip the leading dot for later comparisons
            a = &a[1..];
            b = &b[1..];
        }
        _ => {}
    }

    // Try to strip file extensions
    let (mut a, mut b) = match (remove_file_ending(a), remove_file_ending(b)) {
        (a_stripped, b_stripped) if a_stripped == b_stripped => {
            // If both would be the same after stripping file extensions, don't strip them.
            (a, b)
        }
        stripped => stripped,
    };

    // 1. Compare leading non-numerical part
    // 2. Compare leading numerical part
    // 3. Repeat
    while !a.is_empty() || !b.is_empty() {
        let a_numerical_start = a.iter().position(|c| c.is_ascii_digit()).unwrap_or(a.len());
        let b_numerical_start = b.iter().position(|c| c.is_ascii_digit()).unwrap_or(b.len());

        let a_str = &a[..a_numerical_start];
        let b_str = &b[..b_numerical_start];

        match version_non_digit_cmp(a_str, b_str) {
            Ordering::Equal => {}
            ord => return ord,
        }

        a = &a[a_numerical_start..];
        b = &b[a_numerical_start..];

        let a_numerical_end = a
            .iter()
            .position(|c| !c.is_ascii_digit())
            .unwrap_or(a.len());
        let b_numerical_end = b
            .iter()
            .position(|c| !c.is_ascii_digit())
            .unwrap_or(b.len());

        let a_str = &a[a.iter().position(|&c| c != b'0').unwrap_or(a.len())..a_numerical_end];
        let b_str = &b[b.iter().position(|&c| c != b'0').unwrap_or(b.len())..b_numerical_end];

        match a_str.len().cmp(&b_str.len()) {
            Ordering::Equal => {}
            ord => return ord,
        }

        match a_str.cmp(b_str) {
            Ordering::Equal => {}
            ord => return ord,
        }

        a = &a[a_numerical_end..];
        b = &b[b_numerical_end..];
    }

    Ordering::Equal
}

#[cfg(test)]
mod tests {
    use crate::version_cmp::version_cmp;
    use std::cmp::Ordering;
    #[test]
    fn test_version_cmp() {
        // Identical strings
        assert_eq!(version_cmp(b"hello", b"hello"), Ordering::Equal);

        assert_eq!(version_cmp(b"file12", b"file12"), Ordering::Equal);

        assert_eq!(
            version_cmp(b"file12-suffix", b"file12-suffix"),
            Ordering::Equal
        );

        assert_eq!(
            version_cmp(b"file12-suffix24", b"file12-suffix24"),
            Ordering::Equal
        );

        // Shortened names
        assert_eq!(version_cmp(b"world", b"wo"), Ordering::Greater);

        assert_eq!(version_cmp(b"hello10wo", b"hello10world"), Ordering::Less);

        // Simple names
        assert_eq!(version_cmp(b"world", b"hello"), Ordering::Greater);

        assert_eq!(version_cmp(b"hello", b"world"), Ordering::Less);

        assert_eq!(version_cmp(b"apple", b"ant"), Ordering::Greater);

        assert_eq!(version_cmp(b"ant", b"apple"), Ordering::Less);

        // Uppercase letters
        assert_eq!(
            version_cmp(b"Beef", b"apple"),
            Ordering::Less,
            "Uppercase letters are sorted before all lowercase letters"
        );

        assert_eq!(version_cmp(b"Apple", b"apple"), Ordering::Less);

        assert_eq!(version_cmp(b"apple", b"aPple"), Ordering::Greater);

        // Numbers
        assert_eq!(
            version_cmp(b"100", b"20"),
            Ordering::Greater,
            "Greater numbers are greater even if they start with a smaller digit",
        );

        assert_eq!(
            version_cmp(b"20", b"20"),
            Ordering::Equal,
            "Equal numbers are equal"
        );

        assert_eq!(
            version_cmp(b"15", b"200"),
            Ordering::Less,
            "Small numbers are smaller"
        );

        // Comparing numbers with other characters
        assert_eq!(
            version_cmp(b"1000", b"apple"),
            Ordering::Less,
            "Numbers are sorted before other characters"
        );

        assert_eq!(
            // spell-checker:disable-next-line
            version_cmp(b"file1000", b"fileapple"),
            Ordering::Less,
            "Numbers in the middle of the name are sorted before other characters"
        );

        // Leading zeroes
        assert_eq!(
            version_cmp(b"012", b"12"),
            Ordering::Equal,
            "A single leading zero does not make a difference"
        );

        assert_eq!(
            version_cmp(b"000800", b"0000800"),
            Ordering::Equal,
            "Multiple leading zeros do not make a difference"
        );

        // Numbers and other characters combined
        assert_eq!(version_cmp(b"ab10", b"aa11"), Ordering::Greater);

        assert_eq!(
            version_cmp(b"aa10", b"aa11"),
            Ordering::Less,
            "Numbers after other characters are handled correctly."
        );

        assert_eq!(
            version_cmp(b"aa2", b"aa100"),
            Ordering::Less,
            "Numbers after alphabetical characters are handled correctly."
        );

        assert_eq!(
            version_cmp(b"aa10bb", b"aa11aa"),
            Ordering::Less,
            "Number is used even if alphabetical characters after it differ."
        );

        assert_eq!(
            version_cmp(b"aa10aa0010", b"aa11aa1"),
            Ordering::Less,
            "Second number is ignored if the first number differs."
        );

        assert_eq!(
            version_cmp(b"aa10aa0010", b"aa10aa1"),
            Ordering::Greater,
            "Second number is used if the rest is equal."
        );

        assert_eq!(
            version_cmp(b"aa10aa0010", b"aa00010aa1"),
            Ordering::Greater,
            "Second number is used if the rest is equal up to leading zeroes of the first number."
        );

        assert_eq!(
            version_cmp(b"aa10aa0022", b"aa010aa022"),
            Ordering::Equal,
            "Test multiple numeric values with leading zeros"
        );

        assert_eq!(
            version_cmp(b"file-1.4", b"file-1.13"),
            Ordering::Less,
            "Periods are handled as normal text, not as a decimal point."
        );

        // Greater than u64::Max
        // u64 == 18446744073709551615 so this should be plenty:
        //        20000000000000000000000
        assert_eq!(
            version_cmp(
                b"aa2000000000000000000000bb",
                b"aa002000000000000000000001bb"
            ),
            Ordering::Less,
            "Numbers larger than u64::MAX are handled correctly without crashing"
        );

        assert_eq!(
            version_cmp(
                b"aa2000000000000000000000bb",
                b"aa002000000000000000000000bb"
            ),
            Ordering::Equal,
            "Leading zeroes for numbers larger than u64::MAX are \
            handled correctly without crashing"
        );

        assert_eq!(
            version_cmp(b"  a", b"a"),
            Ordering::Greater,
            "Whitespace is after letters because letters are before non-letters"
        );

        assert_eq!(
            version_cmp(b"a~", b"ab"),
            Ordering::Less,
            "A tilde is before other letters"
        );

        assert_eq!(
            version_cmp(b"a~", b"a"),
            Ordering::Less,
            "A tilde is before the line end"
        );
        assert_eq!(
            version_cmp(b"~", b""),
            Ordering::Greater,
            "A tilde is after the empty string"
        );
        assert_eq!(
            version_cmp(b".f", b".1"),
            Ordering::Greater,
            "if both start with a dot it is ignored for the comparison"
        );

        // The following tests are incompatible with GNU as of 2021/06.
        // I think that's because of a bug in GNU, reported as https://lists.gnu.org/archive/html/bug-coreutils/2021-06/msg00045.html
        assert_eq!(
            version_cmp(b"a..a", b"a.+"),
            Ordering::Less,
            ".a is stripped before the comparison"
        );
        assert_eq!(
            version_cmp(b"a.", b"a+"),
            Ordering::Greater,
            ". is not stripped before the comparison"
        );
        assert_eq!(
            version_cmp(b"a\0a", b"a"),
            Ordering::Greater,
            "NULL bytes are handled comparison"
        );
    }
}
