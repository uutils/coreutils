// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (regex) diuox diuoxX diu oxX

use uucore::format::{Format, FormatError, num_format::UnsignedInt};

use crate::csplit_error::CsplitError;

/// Computes the filename of a split, taking into consideration a possible user-defined suffix
/// format.
pub struct SplitName {
    prefix: Vec<u8>,
    format: Format<UnsignedInt, u64>,
}

impl SplitName {
    /// Creates a new [`SplitName`] with the given user-defined options:
    /// - `prefix_opt` specifies a prefix for all splits.
    /// - `format_opt` specifies a custom format for the suffix part of the filename, using the
    ///   `sprintf` format notation.
    /// - `n_digits_opt` defines the width of the split number.
    ///
    /// # Caveats
    ///
    /// If `prefix_opt` and `format_opt` are defined, and the `format_opt` has some string appearing
    /// before the conversion pattern (e.g., "here-%05d"), then it is appended to the passed prefix
    /// via `prefix_opt`.
    ///
    /// If `n_digits_opt` and `format_opt` are defined, then width defined in `format_opt` is
    /// taken.
    pub fn new(
        prefix_opt: Option<String>,
        format_opt: Option<String>,
        n_digits_opt: Option<String>,
    ) -> Result<Self, CsplitError> {
        // get the prefix
        let prefix = prefix_opt.unwrap_or_else(|| "xx".to_string());

        // the width for the split offset
        let n_digits = n_digits_opt
            .map(|opt| {
                opt.parse::<usize>()
                    .map_err(|_| CsplitError::InvalidNumber(opt))
            })
            .transpose()?
            .unwrap_or(2);

        let format_string = format_opt.unwrap_or_else(|| format!("%0{n_digits}u"));

        check_suffix_format(format_string.as_bytes())?;
        let format = match Format::<UnsignedInt, u64>::parse(format_string) {
            Ok(format) => Ok(format),
            Err(FormatError::TooManySpecs(_)) => Err(CsplitError::SuffixFormatTooManyPercents),
            Err(_) => Err(CsplitError::SuffixFormatIncorrect),
        }?;

        Ok(Self {
            prefix: prefix.as_bytes().to_owned(),
            format,
        })
    }

    /// Returns the filename of the i-th split.
    pub fn get(&self, n: usize) -> String {
        let mut v = self.prefix.clone();
        self.format.fmt(&mut v, n as u64).unwrap();
        String::from_utf8_lossy(&v).to_string()
    }
}

/// Checks a suffix format as GNU csplit does, so that a bad one gets the
/// same message: exactly one conversion, made of `%`, the flags `-`, `0`,
/// `'` and `#`, a width, an optional precision and one of `diuoxX`, where
/// `'` goes only with `diu` and `#` only with `oxX`. `%%` is a literal `%`.
fn check_suffix_format(format: &[u8]) -> Result<(), CsplitError> {
    let digits = |i: &mut usize| {
        while format.get(*i).is_some_and(u8::is_ascii_digit) {
            *i += 1;
        }
    };
    let mut seen = false;
    let mut i = 0;
    while i < format.len() {
        if format[i] != b'%' {
            i += 1;
            continue;
        }
        i += 1;
        if format.get(i) == Some(&b'%') {
            i += 1;
            continue;
        }
        if seen {
            return Err(CsplitError::SuffixFormatTooManyPercents);
        }
        seen = true;
        let (mut quote, mut hash) = (false, false);
        while let Some(&c) = format.get(i) {
            match c {
                b'-' | b'0' => {}
                b'\'' => quote = true,
                b'#' => hash = true,
                _ => break,
            }
            i += 1;
        }
        digits(&mut i);
        if format.get(i) == Some(&b'.') {
            i += 1;
            digits(&mut i);
        }
        let Some(&conversion) = format.get(i) else {
            return Err(CsplitError::SuffixFormatMissingSpecifier);
        };
        let flag = match conversion {
            b'd' | b'i' | b'u' => hash.then_some('#'),
            b'o' | b'x' | b'X' => quote.then_some('\''),
            // GNU prints a byte it cannot show (in the C locale) in octal.
            c if c == b' ' || c.is_ascii_graphic() => {
                return Err(CsplitError::SuffixFormatInvalidSpecifier(
                    char::from(c).to_string(),
                ));
            }
            c => {
                return Err(CsplitError::SuffixFormatInvalidSpecifier(format!(
                    "\\{c:03o}"
                )));
            }
        };
        if let Some(flag) = flag {
            return Err(CsplitError::SuffixFormatInvalidFlags(format!(
                "%{flag}{}",
                char::from(conversion)
            )));
        }
        i += 1;
    }
    if seen {
        Ok(())
    } else {
        Err(CsplitError::SuffixFormatMissingPercent)
    }
}

#[cfg(test)]
mod tests {
    // spell-checker:ignore (path) xxcst

    use super::*;

    #[test]
    fn invalid_number() {
        let split_name = SplitName::new(None, None, Some(String::from("bad")));
        match split_name {
            Err(CsplitError::InvalidNumber(_)) => (),
            _ => panic!("should fail with InvalidNumber"),
        }
    }

    #[test]
    fn invalid_suffix_format1() {
        let split_name = SplitName::new(None, Some(String::from("no conversion string")), None);
        match split_name {
            Err(CsplitError::SuffixFormatMissingPercent) => (),
            _ => panic!("should fail with SuffixFormatMissingPercent"),
        }
    }

    #[test]
    fn invalid_suffix_format2() {
        let split_name = SplitName::new(None, Some(String::from("%042a")), None);
        match split_name {
            Err(CsplitError::SuffixFormatInvalidSpecifier(c)) if c == "a" => (),
            _ => panic!("should fail with SuffixFormatInvalidSpecifier"),
        }
    }

    #[test]
    fn default_formatter() {
        let split_name = SplitName::new(None, None, None).unwrap();
        assert_eq!(split_name.get(2), "xx02");
    }

    #[test]
    fn default_formatter_with_prefix() {
        let split_name = SplitName::new(Some(String::from("aaa")), None, None).unwrap();
        assert_eq!(split_name.get(2), "aaa02");
    }

    #[test]
    fn default_formatter_with_width() {
        let split_name = SplitName::new(None, None, Some(String::from("5"))).unwrap();
        assert_eq!(split_name.get(2), "xx00002");
    }

    #[test]
    fn no_padding_decimal() {
        let split_name = SplitName::new(None, Some(String::from("cst-%d-")), None).unwrap();
        assert_eq!(split_name.get(2), "xxcst-2-");
    }

    #[test]
    fn zero_padding_decimal1() {
        let split_name = SplitName::new(None, Some(String::from("cst-%03d-")), None).unwrap();
        assert_eq!(split_name.get(2), "xxcst-002-");
    }

    #[test]
    fn zero_padding_decimal2() {
        let split_name = SplitName::new(
            Some(String::from("pre-")),
            Some(String::from("cst-%03d-post")),
            None,
        )
        .unwrap();
        assert_eq!(split_name.get(2), "pre-cst-002-post");
    }

    #[test]
    fn zero_padding_decimal3() {
        let split_name = SplitName::new(
            None,
            Some(String::from("cst-%03d-")),
            Some(String::from("42")),
        )
        .unwrap();
        assert_eq!(split_name.get(2), "xxcst-002-");
    }

    #[test]
    fn zero_padding_decimal4() {
        let split_name = SplitName::new(None, Some(String::from("cst-%03i-")), None).unwrap();
        assert_eq!(split_name.get(2), "xxcst-002-");
    }

    #[test]
    fn zero_padding_decimal5() {
        let split_name = SplitName::new(None, Some(String::from("cst-%03u-")), None).unwrap();
        assert_eq!(split_name.get(2), "xxcst-002-");
    }

    #[test]
    fn zero_padding_octal() {
        let split_name = SplitName::new(None, Some(String::from("cst-%03o-")), None).unwrap();
        assert_eq!(split_name.get(42), "xxcst-052-");
    }

    #[test]
    fn zero_padding_lower_hex() {
        let split_name = SplitName::new(None, Some(String::from("cst-%03x-")), None).unwrap();
        assert_eq!(split_name.get(42), "xxcst-02a-");
    }

    #[test]
    fn zero_padding_upper_hex() {
        let split_name = SplitName::new(None, Some(String::from("cst-%03X-")), None).unwrap();
        assert_eq!(split_name.get(42), "xxcst-02A-");
    }

    #[test]
    fn alternate_form_octal() {
        let split_name = SplitName::new(None, Some(String::from("cst-%#10o-")), None).unwrap();
        assert_eq!(split_name.get(42), "xxcst-       052-");
    }

    #[test]
    fn alternate_form_lower_hex() {
        let split_name = SplitName::new(None, Some(String::from("cst-%#10x-")), None).unwrap();
        assert_eq!(split_name.get(42), "xxcst-      0x2a-");
    }

    #[test]
    fn alternate_form_upper_hex() {
        let split_name = SplitName::new(None, Some(String::from("cst-%#10X-")), None).unwrap();
        assert_eq!(split_name.get(42), "xxcst-      0X2A-");
    }

    #[test]
    fn left_adjusted_decimal1() {
        let split_name = SplitName::new(None, Some(String::from("cst-%-10d-")), None).unwrap();
        assert_eq!(split_name.get(42), "xxcst-42        -");
    }

    #[test]
    fn left_adjusted_decimal2() {
        let split_name = SplitName::new(None, Some(String::from("cst-%-10i-")), None).unwrap();
        assert_eq!(split_name.get(42), "xxcst-42        -");
    }

    #[test]
    fn left_adjusted_decimal3() {
        let split_name = SplitName::new(None, Some(String::from("cst-%-10u-")), None).unwrap();
        assert_eq!(split_name.get(42), "xxcst-42        -");
    }

    #[test]
    fn left_adjusted_octal() {
        let split_name = SplitName::new(None, Some(String::from("cst-%-10o-")), None).unwrap();
        assert_eq!(split_name.get(42), "xxcst-52        -");
    }

    #[test]
    fn left_adjusted_lower_hex() {
        let split_name = SplitName::new(None, Some(String::from("cst-%-10x-")), None).unwrap();
        assert_eq!(split_name.get(42), "xxcst-2a        -");
    }

    #[test]
    fn left_adjusted_upper_hex() {
        let split_name = SplitName::new(None, Some(String::from("cst-%-10X-")), None).unwrap();
        assert_eq!(split_name.get(42), "xxcst-2A        -");
    }

    #[test]
    fn suffix_format_errors_name_the_problem() {
        use CsplitError as E;
        let error = |format: &str| SplitName::new(None, Some(format.to_string()), None).err();
        for format in ["abc", "x%%y"] {
            assert!(
                matches!(error(format), Some(E::SuffixFormatMissingPercent)),
                "{format:?}"
            );
        }
        for format in ["a%", "%05."] {
            assert!(
                matches!(error(format), Some(E::SuffixFormatMissingSpecifier)),
                "{format:?}"
            );
        }
        for (format, specifier) in [
            ("%s", "s"),
            ("%ld", "l"),
            ("%\u{1}", "\\001"),
            ("%s%d", "s"),
        ] {
            assert!(
                matches!(error(format), Some(E::SuffixFormatInvalidSpecifier(s)) if s == specifier),
                "{format:?}"
            );
        }
        for (format, specification) in [("%#d", "%#d"), ("%-#5u", "%#u"), ("%'x", "%'x")] {
            assert!(
                matches!(error(format), Some(E::SuffixFormatInvalidFlags(s)) if s == specification),
                "{format:?}"
            );
        }
        assert!(matches!(
            error("%d%s"),
            Some(E::SuffixFormatTooManyPercents)
        ));
    }

    #[test]
    fn suffix_format_flags_gnu_accepts() {
        for format in ["%'d", "%'05i", "%#o", "%-0#8.3X", "100%%-%d", "%%%u%%"] {
            assert!(
                SplitName::new(None, Some(format.to_string()), None).is_ok(),
                "{format:?}"
            );
        }
    }

    #[test]
    fn too_many_percent() {
        let split_name = SplitName::new(None, Some(String::from("%02d-%-3x")), None);
        match split_name {
            Err(CsplitError::SuffixFormatTooManyPercents) => (),
            _ => panic!("should fail with SuffixFormatTooManyPercents"),
        }
    }
}
