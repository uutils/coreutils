use std::cmp::Ordering;
use std::path::Path;

/// Compare paths in a way that matches the GNU version sort, meaning that
/// numbers get sorted in a natural way.
pub(crate) fn version_cmp(a: &Path, b: &Path) -> Ordering {
    let a_string = a.to_string_lossy();
    let b_string = b.to_string_lossy();
    let mut a = a_string.chars().peekable();
    let mut b = b_string.chars().peekable();

    // The order determined from the number of leading zeroes.
    // This is used if the filenames are equivalent up to leading zeroes.
    let mut leading_zeroes = Ordering::Equal;

    loop {
        match (a.next(), b.next()) {
            // If the characters are both numerical. We collect the rest of the number
            // and parse them to u64's and compare them.
            (Some(a_char @ '0'..='9'), Some(b_char @ '0'..='9')) => {
                let mut a_leading_zeroes = 0;
                if a_char == '0' {
                    a_leading_zeroes = 1;
                    while let Some('0') = a.peek() {
                        a_leading_zeroes += 1;
                        a.next();
                    }
                }

                let mut b_leading_zeroes = 0;
                if b_char == '0' {
                    b_leading_zeroes = 1;
                    while let Some('0') = b.peek() {
                        b_leading_zeroes += 1;
                        b.next();
                    }
                }
                // The first different number of leading zeros determines the order
                // so if it's already been determined by a previous number, we leave
                // it as that ordering.
                // It's b.cmp(&a), because the *largest* number of leading zeros
                // should go first
                if leading_zeroes == Ordering::Equal {
                    leading_zeroes = b_leading_zeroes.cmp(&a_leading_zeroes);
                }

                let mut a_str = String::new();
                let mut b_str = String::new();
                if a_char != '0' {
                    a_str.push(a_char);
                }
                if b_char != '0' {
                    b_str.push(b_char);
                }

                // Unwrapping here is fine because we only call next if peek returns
                // Some(_), so next should also return Some(_).
                while let Some('0'..='9') = a.peek() {
                    a_str.push(a.next().unwrap());
                }

                while let Some('0'..='9') = b.peek() {
                    b_str.push(b.next().unwrap());
                }

                // Since the leading zeroes are stripped, the length can be
                // used to compare the numbers.
                match a_str.len().cmp(&b_str.len()) {
                    Ordering::Equal => {}
                    x => return x,
                }

                // At this point, leading zeroes are stripped and the lengths
                // are equal, meaning that the strings can be compared using
                // the standard compare function.
                match a_str.cmp(&b_str) {
                    Ordering::Equal => {}
                    x => return x,
                }
            }
            // If there are two characters we just compare the characters
            (Some(a_char), Some(b_char)) => match a_char.cmp(&b_char) {
                Ordering::Equal => {}
                x => return x,
            },
            // Otherwise, we compare the options (because None < Some(_))
            (a_opt, b_opt) => match a_opt.cmp(&b_opt) {
                // If they are completely equal except for leading zeroes, we use the leading zeroes.
                Ordering::Equal => return leading_zeroes,
                x => return x,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::version_cmp::version_cmp;
    use std::cmp::Ordering;
    use std::path::PathBuf;
    #[test]
    fn test_version_cmp() {
        // Identical strings
        assert_eq!(
            version_cmp(&PathBuf::from("hello"), &PathBuf::from("hello")),
            Ordering::Equal
        );

        assert_eq!(
            version_cmp(&PathBuf::from("file12"), &PathBuf::from("file12")),
            Ordering::Equal
        );

        assert_eq!(
            version_cmp(
                &PathBuf::from("file12-suffix"),
                &PathBuf::from("file12-suffix")
            ),
            Ordering::Equal
        );

        assert_eq!(
            version_cmp(
                &PathBuf::from("file12-suffix24"),
                &PathBuf::from("file12-suffix24")
            ),
            Ordering::Equal
        );

        // Shortened names
        assert_eq!(
            version_cmp(&PathBuf::from("world"), &PathBuf::from("wo")),
            Ordering::Greater,
        );

        assert_eq!(
            version_cmp(&PathBuf::from("hello10wo"), &PathBuf::from("hello10world")),
            Ordering::Less,
        );

        // Simple names
        assert_eq!(
            version_cmp(&PathBuf::from("world"), &PathBuf::from("hello")),
            Ordering::Greater,
        );

        assert_eq!(
            version_cmp(&PathBuf::from("hello"), &PathBuf::from("world")),
            Ordering::Less
        );

        assert_eq!(
            version_cmp(&PathBuf::from("apple"), &PathBuf::from("ant")),
            Ordering::Greater
        );

        assert_eq!(
            version_cmp(&PathBuf::from("ant"), &PathBuf::from("apple")),
            Ordering::Less
        );

        // Uppercase letters
        assert_eq!(
            version_cmp(&PathBuf::from("Beef"), &PathBuf::from("apple")),
            Ordering::Less,
            "Uppercase letters are sorted before all lowercase letters"
        );

        assert_eq!(
            version_cmp(&PathBuf::from("Apple"), &PathBuf::from("apple")),
            Ordering::Less
        );

        assert_eq!(
            version_cmp(&PathBuf::from("apple"), &PathBuf::from("aPple")),
            Ordering::Greater
        );

        // Numbers
        assert_eq!(
            version_cmp(&PathBuf::from("100"), &PathBuf::from("20")),
            Ordering::Greater,
            "Greater numbers are greater even if they start with a smaller digit",
        );

        assert_eq!(
            version_cmp(&PathBuf::from("20"), &PathBuf::from("20")),
            Ordering::Equal,
            "Equal numbers are equal"
        );

        assert_eq!(
            version_cmp(&PathBuf::from("15"), &PathBuf::from("200")),
            Ordering::Less,
            "Small numbers are smaller"
        );

        // Comparing numbers with other characters
        assert_eq!(
            version_cmp(&PathBuf::from("1000"), &PathBuf::from("apple")),
            Ordering::Less,
            "Numbers are sorted before other characters"
        );

        assert_eq!(
            // spell-checker:disable-next-line
            version_cmp(&PathBuf::from("file1000"), &PathBuf::from("fileapple")),
            Ordering::Less,
            "Numbers in the middle of the name are sorted before other characters"
        );

        // Leading zeroes
        assert_eq!(
            version_cmp(&PathBuf::from("012"), &PathBuf::from("12")),
            Ordering::Less,
            "A single leading zero can make a difference"
        );

        assert_eq!(
            version_cmp(&PathBuf::from("000800"), &PathBuf::from("0000800")),
            Ordering::Greater,
            "Leading number of zeroes is used even if both non-zero number of zeros"
        );

        // Numbers and other characters combined
        assert_eq!(
            version_cmp(&PathBuf::from("ab10"), &PathBuf::from("aa11")),
            Ordering::Greater
        );

        assert_eq!(
            version_cmp(&PathBuf::from("aa10"), &PathBuf::from("aa11")),
            Ordering::Less,
            "Numbers after other characters are handled correctly."
        );

        assert_eq!(
            version_cmp(&PathBuf::from("aa2"), &PathBuf::from("aa100")),
            Ordering::Less,
            "Numbers after alphabetical characters are handled correctly."
        );

        assert_eq!(
            version_cmp(&PathBuf::from("aa10bb"), &PathBuf::from("aa11aa")),
            Ordering::Less,
            "Number is used even if alphabetical characters after it differ."
        );

        assert_eq!(
            version_cmp(&PathBuf::from("aa10aa0010"), &PathBuf::from("aa11aa1")),
            Ordering::Less,
            "Second number is ignored if the first number differs."
        );

        assert_eq!(
            version_cmp(&PathBuf::from("aa10aa0010"), &PathBuf::from("aa10aa1")),
            Ordering::Greater,
            "Second number is used if the rest is equal."
        );

        assert_eq!(
            version_cmp(&PathBuf::from("aa10aa0010"), &PathBuf::from("aa00010aa1")),
            Ordering::Greater,
            "Second number is used if the rest is equal up to leading zeroes of the first number."
        );

        assert_eq!(
            version_cmp(&PathBuf::from("aa10aa0022"), &PathBuf::from("aa010aa022")),
            Ordering::Greater,
            "The leading zeroes of the first number has priority."
        );

        assert_eq!(
            version_cmp(&PathBuf::from("aa10aa0022"), &PathBuf::from("aa10aa022")),
            Ordering::Less,
            "The leading zeroes of other numbers than the first are used."
        );

        assert_eq!(
            version_cmp(&PathBuf::from("file-1.4"), &PathBuf::from("file-1.13")),
            Ordering::Less,
            "Periods are handled as normal text, not as a decimal point."
        );

        // Greater than u64::Max
        // u64 == 18446744073709551615 so this should be plenty:
        //        20000000000000000000000
        assert_eq!(
            version_cmp(
                &PathBuf::from("aa2000000000000000000000bb"),
                &PathBuf::from("aa002000000000000000000001bb")
            ),
            Ordering::Less,
            "Numbers larger than u64::MAX are handled correctly without crashing"
        );

        assert_eq!(
        version_cmp(
            &PathBuf::from("aa2000000000000000000000bb"),
            &PathBuf::from("aa002000000000000000000000bb")
        ),
        Ordering::Greater,
        "Leading zeroes for numbers larger than u64::MAX are handled correctly without crashing"
    );
    }
}
