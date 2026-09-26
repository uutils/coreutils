// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Helpers for the argument rewrites GNU accepts but clap does not parse.

use std::ffi::OsString;

/// Split arguments that attach their value to a short option.
///
/// clap keeps only what follows the first `=` of a short option, so GNU's
/// `-t=` and `-d=` do not deliver the value the user typed. Giving the value
/// its own argument sidesteps that, because clap then takes it verbatim.
/// See <https://github.com/uutils/coreutils/issues/2424#issuecomment-863825242>.
///
/// `short` is the option as it is written on the command line, including the
/// leading dash, for example `"-t"`. `-t` on its own, an argument that starts
/// with something else, and `-t` with an empty value are all left alone.
pub fn split_attached_short_value(args: Vec<OsString>, short: &str) -> Vec<OsString> {
    let mut result = Vec::with_capacity(args.len());
    for arg in args {
        // A separator that is not valid UTF-8 is rejected later anyway, so the
        // lossy conversion here only affects arguments that cannot become one.
        let as_str = arg.to_string_lossy();
        match as_str.strip_prefix(short) {
            Some(value) if !value.is_empty() => {
                result.push(OsString::from(short));
                result.push(OsString::from(value));
            }
            _ => result.push(arg),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rewrite(args: &[&str], short: &str) -> Vec<String> {
        let args: Vec<OsString> = args.iter().map(OsString::from).collect();
        split_attached_short_value(args, short)
            .into_iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn splits_an_attached_value() {
        assert_eq!(rewrite(&["-t=", "f1", "f2"], "-t"), ["-t", "=", "f1", "f2"]);
        assert_eq!(rewrite(&["-t:", "f1"], "-t"), ["-t", ":", "f1"]);
    }

    #[test]
    fn keeps_everything_else() {
        assert_eq!(rewrite(&["-t", "=", "f1"], "-t"), ["-t", "=", "f1"]);
        assert_eq!(rewrite(&["-t", "f1"], "-t"), ["-t", "f1"]);
        assert_eq!(rewrite(&["-e", "f1"], "-t"), ["-e", "f1"]);
        assert_eq!(
            rewrite(&["--field-separator="], "-t"),
            ["--field-separator="]
        );
    }
}
