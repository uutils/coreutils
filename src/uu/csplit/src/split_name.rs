// spell-checker:ignore (regex) diuox

use regex::Regex;

use crate::csplit_error::CsplitError;

/// Computes the filename of a split, taking into consideration a possible user-defined suffix
/// format.
pub struct SplitName {
    fn_split_name: Box<dyn Fn(usize) -> String>,
}

impl SplitName {
    /// Creates a new SplitName with the given user-defined options:
    /// - `prefix_opt` specifies a prefix for all splits.
    /// - `format_opt` specifies a custom format for the suffix part of the filename, using the
    /// `sprintf` format notation.
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
        // translate the custom format into a function
        let fn_split_name: Box<dyn Fn(usize) -> String> = match format_opt {
            None => Box::new(move |n: usize| -> String {
                format!("{}{:0width$}", prefix, n, width = n_digits)
            }),
            Some(custom) => {
                let spec =
                    Regex::new(r"(?P<ALL>%((?P<FLAG>[0#-])(?P<WIDTH>\d+)?)?(?P<TYPE>[diuoxX]))")
                        .unwrap();
                let mut captures_iter = spec.captures_iter(&custom);
                let custom_fn: Box<dyn Fn(usize) -> String> = match captures_iter.next() {
                    Some(captures) => {
                        let all = captures.name("ALL").unwrap();
                        let before = custom[0..all.start()].to_owned();
                        let after = custom[all.end()..].to_owned();
                        let n_digits = match captures.name("WIDTH") {
                            None => 0,
                            Some(m) => m.as_str().parse::<usize>().unwrap(),
                        };
                        match (captures.name("FLAG"), captures.name("TYPE")) {
                            (None, Some(ref t)) => match t.as_str() {
                                "d" | "i" | "u" => Box::new(move |n: usize| -> String {
                                    format!("{}{}{}{}", prefix, before, n, after)
                                }),
                                "o" => Box::new(move |n: usize| -> String {
                                    format!("{}{}{:o}{}", prefix, before, n, after)
                                }),
                                "x" => Box::new(move |n: usize| -> String {
                                    format!("{}{}{:x}{}", prefix, before, n, after)
                                }),
                                "X" => Box::new(move |n: usize| -> String {
                                    format!("{}{}{:X}{}", prefix, before, n, after)
                                }),
                                _ => return Err(CsplitError::SuffixFormatIncorrect),
                            },
                            (Some(ref f), Some(ref t)) => {
                                match (f.as_str(), t.as_str()) {
                                    /*
                                     * zero padding
                                     */
                                    // decimal
                                    ("0", "d") | ("0", "i") | ("0", "u") => {
                                        Box::new(move |n: usize| -> String {
                                            format!(
                                                "{}{}{:0width$}{}",
                                                prefix,
                                                before,
                                                n,
                                                after,
                                                width = n_digits
                                            )
                                        })
                                    }
                                    // octal
                                    ("0", "o") => Box::new(move |n: usize| -> String {
                                        format!(
                                            "{}{}{:0width$o}{}",
                                            prefix,
                                            before,
                                            n,
                                            after,
                                            width = n_digits
                                        )
                                    }),
                                    // lower hexadecimal
                                    ("0", "x") => Box::new(move |n: usize| -> String {
                                        format!(
                                            "{}{}{:0width$x}{}",
                                            prefix,
                                            before,
                                            n,
                                            after,
                                            width = n_digits
                                        )
                                    }),
                                    // upper hexadecimal
                                    ("0", "X") => Box::new(move |n: usize| -> String {
                                        format!(
                                            "{}{}{:0width$X}{}",
                                            prefix,
                                            before,
                                            n,
                                            after,
                                            width = n_digits
                                        )
                                    }),

                                    /*
                                     * Alternate form
                                     */
                                    // octal
                                    ("#", "o") => Box::new(move |n: usize| -> String {
                                        format!(
                                            "{}{}{:>#width$o}{}",
                                            prefix,
                                            before,
                                            n,
                                            after,
                                            width = n_digits
                                        )
                                    }),
                                    // lower hexadecimal
                                    ("#", "x") => Box::new(move |n: usize| -> String {
                                        format!(
                                            "{}{}{:>#width$x}{}",
                                            prefix,
                                            before,
                                            n,
                                            after,
                                            width = n_digits
                                        )
                                    }),
                                    // upper hexadecimal
                                    ("#", "X") => Box::new(move |n: usize| -> String {
                                        format!(
                                            "{}{}{:>#width$X}{}",
                                            prefix,
                                            before,
                                            n,
                                            after,
                                            width = n_digits
                                        )
                                    }),

                                    /*
                                     * Left adjusted
                                     */
                                    // decimal
                                    ("-", "d") | ("-", "i") | ("-", "u") => {
                                        Box::new(move |n: usize| -> String {
                                            format!(
                                                "{}{}{:<#width$}{}",
                                                prefix,
                                                before,
                                                n,
                                                after,
                                                width = n_digits
                                            )
                                        })
                                    }
                                    // octal
                                    ("-", "o") => Box::new(move |n: usize| -> String {
                                        format!(
                                            "{}{}{:<#width$o}{}",
                                            prefix,
                                            before,
                                            n,
                                            after,
                                            width = n_digits
                                        )
                                    }),
                                    // lower hexadecimal
                                    ("-", "x") => Box::new(move |n: usize| -> String {
                                        format!(
                                            "{}{}{:<#width$x}{}",
                                            prefix,
                                            before,
                                            n,
                                            after,
                                            width = n_digits
                                        )
                                    }),
                                    // upper hexadecimal
                                    ("-", "X") => Box::new(move |n: usize| -> String {
                                        format!(
                                            "{}{}{:<#width$X}{}",
                                            prefix,
                                            before,
                                            n,
                                            after,
                                            width = n_digits
                                        )
                                    }),

                                    _ => return Err(CsplitError::SuffixFormatIncorrect),
                                }
                            }
                            _ => return Err(CsplitError::SuffixFormatIncorrect),
                        }
                    }
                    None => return Err(CsplitError::SuffixFormatIncorrect),
                };

                // there cannot be more than one format pattern
                if captures_iter.next().is_some() {
                    return Err(CsplitError::SuffixFormatTooManyPercents);
                }
                custom_fn
            }
        };

        Ok(Self { fn_split_name })
    }

    /// Returns the filename of the i-th split.
    pub fn get(&self, n: usize) -> String {
        (self.fn_split_name)(n)
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
        assert_eq!(split_name.get(42), "xxcst-      0o52-");
    }

    #[test]
    fn alternate_form_lower_hex() {
        let split_name = SplitName::new(None, Some(String::from("cst-%#10x-")), None).unwrap();
        assert_eq!(split_name.get(42), "xxcst-      0x2a-");
    }

    #[test]
    fn alternate_form_upper_hex() {
        let split_name = SplitName::new(None, Some(String::from("cst-%#10X-")), None).unwrap();
        assert_eq!(split_name.get(42), "xxcst-      0x2A-");
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
        assert_eq!(split_name.get(42), "xxcst-0o52      -");
    }

    #[test]
    fn left_adjusted_lower_hex() {
        let split_name = SplitName::new(None, Some(String::from("cst-%-10x-")), None).unwrap();
        assert_eq!(split_name.get(42), "xxcst-0x2a      -");
    }

    #[test]
    fn left_adjusted_upper_hex() {
        let split_name = SplitName::new(None, Some(String::from("cst-%-10X-")), None).unwrap();
        assert_eq!(split_name.get(42), "xxcst-0x2A      -");
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
