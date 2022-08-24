//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#![allow(dead_code)]

use rand::distributions::Distribution;
use rand::Rng;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
pub struct AlphanumericNewline;

impl AlphanumericNewline {
    const CHARSET: &'static [u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789\n";
    fn random_char<R>(rng: &mut R) -> char
    where
        R: Rng + ?Sized,
    {
        Self::random(rng) as char
    }

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

pub struct RandomString;

impl RandomString {
    pub fn generate<D>(dist: D, length: usize) -> String
    where
        D: Distribution<u8>,
    {
        rand::thread_rng()
            .sample_iter(dist)
            .take(length)
            .map(|b| b as char)
            .collect()
    }

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
            return String::from("");
        }
        let mut result = String::from("");
        let mut samples = if end_with_delimiter {
            num_delimiter.max(1)
        } else {
            num_delimiter + 1
        };
        let characters_per_sample = length / samples;
        while samples != 0 {
            result.extend(
                rand::thread_rng()
                    .sample_iter(&dist)
                    .take(characters_per_sample)
                    .map(|b| b as char),
            );
            result.push(delimiter as char);
            samples -= 1;
        }

        if end_with_delimiter {
            let mut string = result
                .bytes()
                .take(length - 1)
                .map(|b| b as char)
                .collect::<String>();
            string.push(delimiter as char);
            string
        } else {
            result.bytes().take(length).map(|b| b as char).collect()
        }
    }
}
