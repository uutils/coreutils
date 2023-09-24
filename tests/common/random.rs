// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use rand::distributions::{Distribution, Uniform};
use rand::{thread_rng, Rng};

/// Samples alphanumeric characters `[A-Za-z0-9]` including newline `\n`
///
/// # Examples
///
/// ```rust,ignore
/// use rand::{Rng, thread_rng};
///
/// let vec = thread_rng()
///     .sample_iter(AlphanumericNewline)
///     .take(10)
///     .collect::<Vec<u8>>();
/// println!("Random chars: {}", String::from_utf8(vec).unwrap());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct AlphanumericNewline;

impl AlphanumericNewline {
    /// The charset to act upon
    const CHARSET: &'static [u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789\n";

    /// Generate a random byte from [`Self::CHARSET`] and return it as `u8`.
    ///
    /// # Arguments
    ///
    /// * `rng`: A [`rand::Rng`]
    ///
    /// returns: u8
    fn random<R>(rng: &mut R) -> u8
    where
        R: Rng + ?Sized,
    {
        let idx = rng.gen_range(0..Self::CHARSET.len());
        Self::CHARSET[idx]
    }
}

impl Distribution<u8> for AlphanumericNewline {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> u8 {
        Self::random(rng)
    }
}

/// Generate a random string from a [`Distribution`]
///
/// # Examples
///
/// ```rust,ignore
/// use crate::common::random::{AlphanumericNewline, RandomString};
/// use rand::distributions::Alphanumeric;
///
/// // generates a 100 byte string with characters from AlphanumericNewline
/// let random_string = RandomString::generate(AlphanumericNewline, 100);
/// assert_eq!(100, random_string.len());
///
/// // generates a 100 byte string with 10 newline characters not ending with a newline
/// let string = RandomString::generate_with_delimiter(Alphanumeric, b'\n', 10, false, 100);
/// assert_eq!(100, random_string.len());
/// ```
pub struct RandomString;

impl RandomString {
    /// Generate a random string from the given [`Distribution`] with the given `length` in bytes.
    ///
    /// # Arguments
    ///
    /// * `dist`: A u8 [`Distribution`]
    /// * `length`: the length of the resulting string in bytes
    ///
    /// returns: String
    pub fn generate<D>(dist: D, length: usize) -> String
    where
        D: Distribution<u8>,
    {
        thread_rng()
            .sample_iter(dist)
            .take(length)
            .map(|b| b as char)
            .collect()
    }

    /// Generate a random string from the [`Distribution`] with the given `length` in bytes. The
    /// function takes a `delimiter`, which is randomly distributed in the string, such that exactly
    /// `num_delimiter` amount of `delimiter`s occur. If `end_with_delimiter` is set, then the
    /// string ends with the delimiter, else the string does not end with the delimiter.
    ///
    /// # Arguments
    ///
    /// * `dist`: A `u8` [`Distribution`]
    /// * `delimiter`: A `u8` delimiter, which does not need to be included in the `Distribution`
    /// * `num_delimiter`: The number of `delimiter`s contained in the resulting string
    /// * `end_with_delimiter`: If the string shall end with the given delimiter
    /// * `length`: the length of the resulting string in bytes
    ///
    /// returns: String
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::common::random::{AlphanumericNewline, RandomString};
    ///
    /// // generates a 100 byte string with 10 '\0' byte characters not ending with a '\0' byte
    /// let string = RandomString::generate_with_delimiter(AlphanumericNewline, 0, 10, false, 100);
    /// assert_eq!(100, random_string.len());
    /// assert_eq!(
    ///     10,
    ///     random_string.as_bytes().iter().filter(|p| **p == 0).count()
    /// );
    /// assert!(!random_string.as_bytes().ends_with(&[0]));
    /// ```
    pub fn generate_with_delimiter<D>(
        dist: D,
        delimiter: u8,
        num_delimiter: usize,
        end_with_delimiter: bool,
        length: usize,
    ) -> String
    where
        D: Distribution<u8>,
    {
        if length == 0 {
            return String::new();
        } else if length == 1 {
            return if num_delimiter > 0 {
                String::from(delimiter as char)
            } else {
                String::from(thread_rng().sample(&dist) as char)
            };
        }

        let samples = length - 1;
        let mut result: Vec<u8> = thread_rng().sample_iter(&dist).take(samples).collect();

        if num_delimiter == 0 {
            result.push(thread_rng().sample(&dist));
            return String::from_utf8(result).unwrap();
        }

        let num_delimiter = if end_with_delimiter {
            num_delimiter - 1
        } else {
            num_delimiter
        };

        let between = Uniform::new(0, samples);
        for _ in 0..num_delimiter {
            let mut pos = between.sample(&mut thread_rng());
            let turn = pos;
            while result[pos] == delimiter {
                pos += 1;
                if pos >= samples {
                    pos = 0;
                }
                if pos == turn {
                    break;
                }
            }
            result[pos] = delimiter;
        }

        if end_with_delimiter {
            result.push(delimiter);
        } else {
            result.push(thread_rng().sample(&dist));
        }

        String::from_utf8(result).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::distributions::Alphanumeric;

    #[test]
    fn test_random_string_generate() {
        let random_string = RandomString::generate(AlphanumericNewline, 0);
        assert_eq!(0, random_string.len());

        let random_string = RandomString::generate(AlphanumericNewline, 1);
        assert_eq!(1, random_string.len());

        let random_string = RandomString::generate(AlphanumericNewline, 100);
        assert_eq!(100, random_string.len());
    }

    #[test]
    fn test_random_string_generate_with_delimiter_when_length_is_zero() {
        let random_string = RandomString::generate_with_delimiter(Alphanumeric, 0, 0, false, 0);
        assert_eq!(0, random_string.len());
    }

    #[test]
    fn test_random_string_generate_with_delimiter_when_num_delimiter_is_greater_than_length() {
        let random_string = RandomString::generate_with_delimiter(Alphanumeric, 0, 2, false, 1);
        assert_eq!(1, random_string.len());
        assert!(random_string.as_bytes().contains(&0));
        assert!(random_string.as_bytes().ends_with(&[0]));
    }

    #[test]
    fn test_random_string_generate_with_delimiter_should_end_with_delimiter() {
        let random_string = RandomString::generate_with_delimiter(Alphanumeric, 0, 1, true, 1);
        assert_eq!(1, random_string.len());
        assert_eq!(
            1,
            random_string.as_bytes().iter().filter(|p| **p == 0).count()
        );
        assert!(random_string.as_bytes().ends_with(&[0]));

        let random_string = RandomString::generate_with_delimiter(Alphanumeric, 0, 1, false, 1);
        assert_eq!(1, random_string.len());
        assert_eq!(
            1,
            random_string.as_bytes().iter().filter(|p| **p == 0).count()
        );
        assert!(random_string.as_bytes().ends_with(&[0]));

        let random_string = RandomString::generate_with_delimiter(Alphanumeric, 0, 1, true, 2);
        assert_eq!(2, random_string.len());
        assert_eq!(
            1,
            random_string.as_bytes().iter().filter(|p| **p == 0).count()
        );
        assert!(random_string.as_bytes().ends_with(&[0]));

        let random_string = RandomString::generate_with_delimiter(Alphanumeric, 0, 2, true, 2);
        assert_eq!(2, random_string.len());
        assert_eq!(
            2,
            random_string.as_bytes().iter().filter(|p| **p == 0).count()
        );
        assert!(random_string.as_bytes().ends_with(&[0]));

        let random_string = RandomString::generate_with_delimiter(Alphanumeric, 0, 1, true, 3);
        assert_eq!(3, random_string.len());
        assert_eq!(
            1,
            random_string.as_bytes().iter().filter(|p| **p == 0).count()
        );
        assert!(random_string.as_bytes().ends_with(&[0]));
    }

    #[test]
    fn test_random_string_generate_with_delimiter_should_not_end_with_delimiter() {
        let random_string = RandomString::generate_with_delimiter(Alphanumeric, 0, 0, false, 1);
        assert_eq!(1, random_string.len());
        assert_eq!(
            0,
            random_string.as_bytes().iter().filter(|p| **p == 0).count()
        );

        let random_string = RandomString::generate_with_delimiter(Alphanumeric, 0, 0, true, 1);
        assert_eq!(1, random_string.len());
        assert_eq!(
            0,
            random_string.as_bytes().iter().filter(|p| **p == 0).count()
        );

        let random_string = RandomString::generate_with_delimiter(Alphanumeric, 0, 1, false, 2);
        assert_eq!(2, random_string.len());
        assert_eq!(
            1,
            random_string.as_bytes().iter().filter(|p| **p == 0).count()
        );
        assert!(!random_string.as_bytes().ends_with(&[0]));

        let random_string = RandomString::generate_with_delimiter(Alphanumeric, 0, 1, false, 3);
        assert_eq!(3, random_string.len());
        assert_eq!(
            1,
            random_string.as_bytes().iter().filter(|p| **p == 0).count()
        );
        assert!(!random_string.as_bytes().ends_with(&[0]));

        let random_string = RandomString::generate_with_delimiter(Alphanumeric, 0, 2, false, 3);
        assert_eq!(3, random_string.len());
        assert_eq!(
            2,
            random_string.as_bytes().iter().filter(|p| **p == 0).count()
        );
        assert!(!random_string.as_bytes().ends_with(&[0]));
    }

    #[test]
    fn test_generate_with_delimiter_with_greater_length() {
        let random_string =
            RandomString::generate_with_delimiter(Alphanumeric, 0, 100, false, 1000);
        assert_eq!(1000, random_string.len());
        assert_eq!(
            100,
            random_string.as_bytes().iter().filter(|p| **p == 0).count()
        );
        assert!(!random_string.as_bytes().ends_with(&[0]));

        let random_string = RandomString::generate_with_delimiter(Alphanumeric, 0, 100, true, 1000);
        assert_eq!(1000, random_string.len());
        assert_eq!(
            100,
            random_string.as_bytes().iter().filter(|p| **p == 0).count()
        );
        assert!(random_string.as_bytes().ends_with(&[0]));
    }

    /// Originally used to exclude an error within the `random` module. The two
    /// affected tests timed out on windows, but only in the ci. These tests are
    /// also the source for the concrete numbers. The timed out tests are
    /// `test_tail.rs::test_pipe_when_lines_option_given_input_size_has_multiple_size_of_buffer_size`
    /// `test_tail.rs::test_pipe_when_bytes_option_given_input_size_has_multiple_size_of_buffer_size`.
    #[test]
    fn test_generate_random_strings_when_length_is_around_critical_buffer_sizes() {
        let length = 8192 * 3;
        let random_string = RandomString::generate(AlphanumericNewline, length);
        assert_eq!(length, random_string.len());

        let length = 8192 * 3 + 1;
        let random_string =
            RandomString::generate_with_delimiter(Alphanumeric, b'\n', 100, true, length);
        assert_eq!(length, random_string.len());
        assert_eq!(
            100,
            random_string
                .as_bytes()
                .iter()
                .filter(|p| **p == b'\n')
                .count()
        );
        assert!(!random_string.as_bytes().ends_with(&[0]));
    }
}
