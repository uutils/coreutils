// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (regex) diuox

use uucore::format::{num_format::UnsignedInt, Format, FormatError};

use crate::csplit_error::CsplitError;

/// Computes the filename of a split, taking into consideration a possible user-defined suffix
/// format.
pub struct SplitName {
    prefix: Vec<u8>,
    format: Format<UnsignedInt>,
}

impl SplitName {
    /// Creates a new SplitName with the given user-defined options:
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

        let format_string = match format_opt {
            Some(f) => f,
            None => format!("%0{n_digits}u"),
        };

        let format = match Format::<UnsignedInt>::parse(format_string) {
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
        };
    }

    #[test]
    fn invalid_suffix_format1() {
        let split_name = SplitName::new(None, Some(String::from("no conversion string")), None);
        match split_name {
            Err(CsplitError::SuffixFormatIncorrect) => (),
            _ => panic!("should fail with SuffixFormatIncorrect"),
        };
    }

    #[test]
    fn invalid_suffix_format2() {
        let split_name = SplitName::new(None, Some(String::from("%042a")), None);
        match split_name {
            Err(CsplitError::SuffixFormatIncorrect) => (),
            _ => panic!("should fail with SuffixFormatIncorrect"),
        };
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
    fn too_many_percent() {
        let split_name = SplitName::new(None, Some(String::from("%02d-%-3x")), None);
        match split_name {
            Err(CsplitError::SuffixFormatTooManyPercents) => (),
            _ => panic!("should fail with SuffixFormatTooManyPercents"),
        };
    }
}
