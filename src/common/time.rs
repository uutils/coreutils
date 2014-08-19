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
        return Err("empty string".to_string())
    }
    let slice = string.slice_to(len - 1);
    let (numstr, times) = match string.char_at(len - 1) {
        's' | 'S' => (slice, 1u),
        'm' | 'M' => (slice, 60u),
        'h' | 'H' => (slice, 60u * 60),
        'd' | 'D' => (slice, 60u * 60 * 24),
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
    match ::std::from_str::from_str::<f64>(numstr) {
        Some(m) => Ok(m * times as f64),
        None => Err(format!("invalid time interval '{}'", string))
    }
}
