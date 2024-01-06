// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
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
            None => Box::new(move |n: usize| -> String { format!("{prefix}{n:0n_digits$}") }),
            Some(custom) => {
                let spec =
                    Regex::new(r"(?P<ALL>%(?P<FLAGS>[#0-]*)(?P<WIDTH>[0-9]+)?(.(?P<PRECISION>[0-9]?))?(?P<CONVERSION>[diuoxX]))")
                        .unwrap();
                let mut captures_iter = spec.captures_iter(&custom);
                let custom_fn: Box<dyn Fn(usize) -> String> = match captures_iter.next() {
                    Some(captures) => {
                        let all = captures.name("ALL").unwrap();
                        let before = custom[0..all.start()].to_owned();
                        let after = custom[all.end()..].to_owned();
                        let width = match captures.name("WIDTH") {
                            None => 0,
                            Some(m) => m.as_str().parse::<usize>().unwrap(),
                        };

                        match (captures.name("FLAGS"), captures.name("CONVERSION")) {
                            (None, Some(ref t)) => match t.as_str() {
                                "d" | "i" | "u" => Box::new(move |n: usize| -> String {
                                    format!("{prefix}{before}{n}{after}")
                                }),
                                "o" => Box::new(move |n: usize| -> String {
                                    format!("{prefix}{before}{n:o}{after}")
                                }),
                                "x" => Box::new(move |n: usize| -> String {
                                    format!("{prefix}{before}{n:x}{after}")
                                }),
                                "X" => Box::new(move |n: usize| -> String {
                                    format!("{prefix}{before}{n:X}{after}")
                                }),
                                _ => return Err(CsplitError::SuffixFormatIncorrect),
                            },
                            (Some(matched_flags), Some(ref matched_conversion)) => {
                                let flags = matched_flags.as_str().to_owned();
                                let conversion = matched_conversion.as_str().to_owned();

                                let mut flag_alternative = false;
                                let mut flag_zero = false;
                                let mut flag_minus = false;
                                for char in flags.chars() {
                                    match char {
                                        '#' => flag_alternative = true,
                                        '0' => flag_zero = true,
                                        '-' => flag_minus = true,
                                        _ => unreachable!(
                                            "Flags should be already filtered by the regex: received {char}"
                                        ),
                                    }
                                }

                                // Interaction between flags: minus cancels zero
                                if flag_minus {
                                    flag_zero = false;
                                }

                                // Alternative flag is not compatible with decimal conversions
                                if (conversion == "d" || conversion == "i" || conversion == "u")
                                    && flag_alternative
                                {
                                    return Err(CsplitError::SuffixFormatIncorrect);
                                }

                                let precision = match captures.name("PRECISION") {
                                    Some(m) => {
                                        // precision cancels the flag_zero
                                        flag_zero = false;
                                        let precision_str = m.as_str();
                                        // only one dot could be given
                                        // in this case, default precision becomes 0
                                        if precision_str.is_empty() {
                                            0
                                        } else {
                                            precision_str.parse::<usize>().unwrap()
                                        }
                                    }
                                    None => {
                                        //default precision is 1 for d,i,u,o,x,X
                                        1
                                    }
                                };

                                Box::new(move |n: usize| -> String {
                                    // First step: Formatting the number with precision, zeros, alternative style...
                                    let precision_formatted =  match conversion.as_str() {
                                            "d" | "i" | "u" => match (n, precision) {
                                                (0, 0) => String::new(),
                                                (_, _) => format!("{n:0precision$}")
                                            }
                                            "o" => match (n, flag_alternative, precision) {
                                                (0, true, _) => format!("{n:0>precision$o}"),
                                                (0, false, 0) => String::new(),
                                                (_, true, 0) => format!("0{n:o}"),
                                                (_, true, _) => format!(
                                                        "{:0>precision$}",
                                                        format!("0{n:o}")
                                                ),
                                                (_, false, _) => format!("{n:0precision$o}"),
                                            }
                                            "x" => match (n, flag_alternative, precision) {
                                                (0, _, 0) => String::new(),
                                                (0, true, _) => format!("{n:0precision$x}"),
                                                ( _,true, _) => format!("{n:#0size$x}", size = precision + 2 ),
                                                ( _,false, 0) => format!("{n:precision$x}"),
                                                (_, _, _) => format!("{n:0precision$x}")
                                            }
                                            "X" => match (n, flag_alternative, precision) {
                                                (0, _, 0) => String::new(),
                                                (0, true, _) => format!("{n:0precision$X}"),
                                                ( _,true, _) => format!("{n:#0size$X}", size = precision + 2 ),
                                                ( _,false, 0) => format!("{n:precision$X}"),
                                                (_, _, _) => format!("{n:0precision$X}")
                                            }
                                            _ => unreachable!("Conversion are filtered by the regex : received {conversion}"),
                                        }
                                    ;

                                    // second step : Fit the number in the width with correct padding and filling
                                    let width_formatted = match (flag_minus, flag_zero) {
                                        (true, true) => format!("{precision_formatted:0<width$}"),
                                        (true, false) => format!("{precision_formatted:<width$}"),
                                        (false, true) => format!("{precision_formatted:0>width$}"),
                                        (false, false) => format!("{precision_formatted:>width$}"),
                                    };

                                    format!("{prefix}{before}{width_formatted}{after}")
                                })
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
    fn invalid_suffix_format_plus() {
        let split_name = SplitName::new(None, Some(String::from("%+")), None);
        match split_name {
            Err(CsplitError::SuffixFormatIncorrect) => (),
            _ => panic!("should fail with SuffixFormatIncorrect"),
        };
    }

    #[test]
    fn invalid_suffix_format_space() {
        let split_name = SplitName::new(None, Some(String::from("% ")), None);
        match split_name {
            Err(CsplitError::SuffixFormatIncorrect) => (),
            _ => panic!("should fail with SuffixFormatIncorrect"),
        };
    }

    #[test]
    fn invalid_suffix_format_alternative_decimal() {
        let split_name = SplitName::new(None, Some(String::from("%#d")), None);
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
        assert_eq!(split_name.get(0), "xxcst-000-");
        assert_eq!(split_name.get(1), "xxcst-001-");
        assert_eq!(split_name.get(42), "xxcst-052-");
    }

    #[test]
    fn zero_padding_lower_hex() {
        let split_name = SplitName::new(None, Some(String::from("cst-%03x-")), None).unwrap();
        assert_eq!(split_name.get(0), "xxcst-000-");
        assert_eq!(split_name.get(1), "xxcst-001-");
        assert_eq!(split_name.get(42), "xxcst-02a-");
    }

    #[test]
    fn zero_padding_upper_hex() {
        let split_name = SplitName::new(None, Some(String::from("cst-%03X-")), None).unwrap();
        assert_eq!(split_name.get(0), "xxcst-000-");
        assert_eq!(split_name.get(1), "xxcst-001-");
        assert_eq!(split_name.get(42), "xxcst-02A-");
    }

    #[test]
    fn alternate_form_octal() {
        let split_name = SplitName::new(None, Some(String::from("cst-%#10o-")), None).unwrap();
        assert_eq!(split_name.get(0), "xxcst-         0-");
        assert_eq!(split_name.get(1), "xxcst-        01-");
        assert_eq!(split_name.get(42), "xxcst-       052-");
    }

    #[test]
    fn form_lower_hex_width() {
        let split_name = SplitName::new(None, Some(String::from("cst-%06x-")), None).unwrap();
        assert_eq!(split_name.get(0), "xxcst-000000-");
        assert_eq!(split_name.get(1), "xxcst-000001-");
        assert_eq!(split_name.get(42), "xxcst-00002a-");
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
    fn alternate_form_lower_hex_precision0() {
        let split_name = SplitName::new(None, Some(String::from("cst-%#6.0x-")), None).unwrap();
        assert_eq!(split_name.get(0), "xxcst-      -");
        assert_eq!(split_name.get(1), "xxcst-   0x1-");
        assert_eq!(split_name.get(42), "xxcst-  0x2a-");
    }

    #[test]
    fn alternate_form_lower_hex_precision1() {
        let split_name = SplitName::new(None, Some(String::from("cst-%#6.1x-")), None).unwrap();
        assert_eq!(split_name.get(0), "xxcst-     0-");
        assert_eq!(split_name.get(1), "xxcst-   0x1-");
        assert_eq!(split_name.get(2), "xxcst-   0x2-");
        assert_eq!(split_name.get(42), "xxcst-  0x2a-");
    }

    #[test]
    fn alternate_form_lower_hex_precision2() {
        let split_name = SplitName::new(None, Some(String::from("cst-%#6.2x-")), None).unwrap();
        assert_eq!(split_name.get(0), "xxcst-    00-");
        assert_eq!(split_name.get(1), "xxcst-  0x01-");
        assert_eq!(split_name.get(2), "xxcst-  0x02-");
        assert_eq!(split_name.get(42), "xxcst-  0x2a-");
    }

    #[test]
    fn alternate_form_lower_hex_precision3() {
        let split_name = SplitName::new(None, Some(String::from("cst-%#6.3x-")), None).unwrap();
        assert_eq!(split_name.get(0), "xxcst-   000-");
        assert_eq!(split_name.get(1), "xxcst- 0x001-");
        assert_eq!(split_name.get(2), "xxcst- 0x002-");
        assert_eq!(split_name.get(42), "xxcst- 0x02a-");
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
    fn left_adjusted_decimal_precision() {
        let split_name = SplitName::new(None, Some(String::from("cst-%-10.3u-")), None).unwrap();
        assert_eq!(split_name.get(0), "xxcst-000       -");
        assert_eq!(split_name.get(1), "xxcst-001       -");
        assert_eq!(split_name.get(42), "xxcst-042       -");
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
        // assert_eq!(split_name.get(42), "xxcst-0x2A      -");
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

    #[test]
    fn precision_decimal0() {
        let split_name = SplitName::new(None, Some(String::from("cst-%3.0u-")), None).unwrap();
        assert_eq!(split_name.get(0), "xxcst-   -");
        assert_eq!(split_name.get(1), "xxcst-  1-");
        assert_eq!(split_name.get(2), "xxcst-  2-");
    }

    #[test]
    fn precision_decimal1() {
        let split_name = SplitName::new(None, Some(String::from("cst-%3.1u-")), None).unwrap();
        assert_eq!(split_name.get(0), "xxcst-  0-");
        assert_eq!(split_name.get(1), "xxcst-  1-");
        assert_eq!(split_name.get(2), "xxcst-  2-");
    }

    #[test]
    fn alternate_octal() {
        let split_name = SplitName::new(None, Some(String::from("%#6o")), None).unwrap();
        assert_eq!(split_name.get(0), "xx     0");
        assert_eq!(split_name.get(1), "xx    01");
    }

    #[test]
    fn precision_octal0() {
        let split_name = SplitName::new(None, Some(String::from("%.0o")), None).unwrap();
        assert_eq!(split_name.get(0), "xx");
        assert_eq!(split_name.get(1), "xx1");
    }

    #[test]
    fn precision_octal1() {
        let split_name = SplitName::new(None, Some(String::from("%.1o")), None).unwrap();
        assert_eq!(split_name.get(0), "xx0");
        assert_eq!(split_name.get(1), "xx1");
    }

    #[test]
    fn precision_octal3() {
        let split_name = SplitName::new(None, Some(String::from("%.3o")), None).unwrap();
        assert_eq!(split_name.get(0), "xx000");
        assert_eq!(split_name.get(1), "xx001");
    }

    #[test]
    fn precision_lower_hex0() {
        let split_name = SplitName::new(None, Some(String::from("%.0x")), None).unwrap();
        assert_eq!(split_name.get(0), "xx");
        assert_eq!(split_name.get(1), "xx1");
    }

    #[test]
    fn precision_lower_hex1() {
        let split_name = SplitName::new(None, Some(String::from("%.1x")), None).unwrap();
        assert_eq!(split_name.get(0), "xx0");
        assert_eq!(split_name.get(1), "xx1");
    }

    #[test]
    fn precision_lower_hex3() {
        let split_name = SplitName::new(None, Some(String::from("%.3x")), None).unwrap();
        assert_eq!(split_name.get(0), "xx000");
        assert_eq!(split_name.get(1), "xx001");
    }

    #[test]
    fn precision_upper_hex0() {
        let split_name = SplitName::new(None, Some(String::from("%.0x")), None).unwrap();
        assert_eq!(split_name.get(0), "xx");
        assert_eq!(split_name.get(1), "xx1");
    }

    #[test]
    fn precision_upper_hex1() {
        let split_name = SplitName::new(None, Some(String::from("%.1x")), None).unwrap();
        assert_eq!(split_name.get(0), "xx0");
        assert_eq!(split_name.get(1), "xx1");
    }

    #[test]
    fn precision_upper_hex3() {
        let split_name = SplitName::new(None, Some(String::from("%.3x")), None).unwrap();
        assert_eq!(split_name.get(0), "xx000");
        assert_eq!(split_name.get(1), "xx001");
    }

    #[test]
    fn precision_alternate_lower_hex0() {
        let split_name = SplitName::new(None, Some(String::from("%#10.0x")), None).unwrap();
        assert_eq!(split_name.get(0), "xx          ");
        assert_eq!(split_name.get(1), "xx       0x1");
    }

    #[test]
    fn precision_alternate_lower_hex1() {
        let split_name = SplitName::new(None, Some(String::from("%#10.1x")), None).unwrap();
        assert_eq!(split_name.get(0), "xx         0");
        assert_eq!(split_name.get(1), "xx       0x1");
    }

    #[test]
    fn precision_alternate_lower_hex2() {
        let split_name = SplitName::new(None, Some(String::from("%#10.2x")), None).unwrap();
        assert_eq!(split_name.get(0), "xx        00");
        assert_eq!(split_name.get(1), "xx      0x01");
    }

    #[test]
    fn precision_alternate_octal0() {
        let split_name = SplitName::new(None, Some(String::from("%#6.0o")), None).unwrap();
        assert_eq!(split_name.get(0), "xx     0");
        assert_eq!(split_name.get(1), "xx    01");
    }

    #[test]
    fn precision_alternate_octal1() {
        let split_name = SplitName::new(None, Some(String::from("%#6.1o")), None).unwrap();
        assert_eq!(split_name.get(0), "xx     0");
        assert_eq!(split_name.get(1), "xx    01");
    }

    #[test]
    fn precision_alternate_octal2() {
        let split_name = SplitName::new(None, Some(String::from("%#6.2o")), None).unwrap();
        assert_eq!(split_name.get(0), "xx    00");
        assert_eq!(split_name.get(1), "xx    01");
    }

    #[test]
    fn precision_alternate_octal3() {
        let split_name = SplitName::new(None, Some(String::from("%#6.3o")), None).unwrap();
        assert_eq!(split_name.get(0), "xx   000");
        assert_eq!(split_name.get(1), "xx   001");
    }

    #[test]
    fn precision_only_dot_decimal() {
        // if only one dot is given, precision becomes 0
        let split_name = SplitName::new(None, Some(String::from("%.u")), None).unwrap();
        assert_eq!(split_name.get(0), "xx");
        assert_eq!(split_name.get(1), "xx1");
        assert_eq!(split_name.get(42), "xx42");
    }
    #[test]
    fn precision_only_dot_octal() {
        // if only one dot is given, precision becomes 0
        let split_name = SplitName::new(None, Some(String::from("%.o")), None).unwrap();
        assert_eq!(split_name.get(0), "xx");
        assert_eq!(split_name.get(1), "xx1");
        assert_eq!(split_name.get(42), "xx52");
    }
    #[test]
    fn precision_only_dot_lower_hex() {
        // if only one dot is given, precision becomes 0
        let split_name = SplitName::new(None, Some(String::from("%.x")), None).unwrap();
        assert_eq!(split_name.get(0), "xx");
        assert_eq!(split_name.get(1), "xx1");
        assert_eq!(split_name.get(42), "xx2a");
    }
    #[test]
    fn precision_only_dot_upper_hex() {
        // if only one dot is given, precision becomes 0
        let split_name = SplitName::new(None, Some(String::from("%.X")), None).unwrap();
        assert_eq!(split_name.get(0), "xx");
        assert_eq!(split_name.get(1), "xx1");
        assert_eq!(split_name.get(42), "xx2A");
    }
}
