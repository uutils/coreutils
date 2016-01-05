/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

pub fn from_str(string: &str) -> Result<f64, String> {
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
    match numstr.parse::<f64>() {
        Ok(m) => Ok(m * times as f64),
        Err(e) => Err(format!("invalid time interval '{}': {}", string, e))
    }
}
