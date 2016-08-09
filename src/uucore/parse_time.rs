/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use std::time::Duration;

pub fn from_str(string: &str) -> Result<Duration, String> {
    let len = string.len();
    if len == 0 {
        return Err("empty string".to_owned())
    }
    let slice = &string[..len - 1];
    let (numstr, times) = match string.chars().next_back().unwrap() {
        's' | 'S' => (slice, 1),
        'm' | 'M' => (slice, 60),
        'h' | 'H' => (slice, 60 * 60),
        'd' | 'D' => (slice, 60 * 60 * 24),
        val => {
            if !val.is_alphabetic() {
                (string, 1)
            } else if string == "inf" || string == "infinity" {
                ("inf", 1)
            } else {
                return Err(format!("invalid time interval '{}'", string))
            }
        }
    };
    let num = match numstr.parse::<f64>() {
        Ok(m) => m,
        Err(e) => return Err(format!("invalid time interval '{}': {}", string, e))
    };

    const NANOS_PER_SEC: u32 = 1_000_000_000;
    let whole_secs = num.trunc();
    let nanos = (num.fract() * (NANOS_PER_SEC as f64)).trunc();
    let duration = Duration::new(whole_secs as u64, nanos as u32);
    Ok(duration * times)
}
