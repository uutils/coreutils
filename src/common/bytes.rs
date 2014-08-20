/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Colin Warren <me@zv.ms>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/// Parse strings in the form XXXMB
/// Supported suffixes:
///   * c  (?)         = 1
///   * w  (x86 word)  = 2
///   * b  (block)     = 512
///   * kB (kilobyte)  = 1000
///   * K  (kibibyte)  = 1024
///   * MB (megabyte)  = kB * 1000
///   * M  (mebibyte)  = K  * 1024
///   * xM (mebibyte)  = M
///   * GB (gigabyte)  = MB * 1000
///   * G  (gibibyte)  = M  * 1024
///   * TB (terabyte)  = GB * 1000
///   * T  (tebibyte)  = G  * 1024
///   * PB (petabyte)  = TB * 1000
///   * P  (pebibyte)  = T  * 1024
///   * EB (exabyte)   = PB * 1000
///   * E  (exbibyte)  = P  * 1024
///   * ZB (zettabyte) = EB * 1000
///   * Z  (zebibyte)  = E  * 1024
///   * YB (yottabyte) = ZB * 1000
///   * Y  (yobibyte)  = Z  * 1024
pub fn from_human(bytes: &str) -> Result<u64, String> {
    let num_str = bytes.chars().take_while(|c| c.is_digit()).collect::<String>();
    let suffix = bytes.chars().skip(num_str.len()).collect::<String>();

    if num_str.len() == 0 {
        return Err("invalid bytes".to_string());
    }

    let multiplier = match suffix.as_slice() {
        "" => 1,
        "c" => 1,
        "w" => 2,
        "b" => 512,
        "kB" => 1000,
        "K" => 1024,
        "MB" => 1000 * 1000,
        "M" => 1024 * 1024,
        "xM" => 1024 * 1024,
        "GB" => 1000 * 1000 * 1000,
        "G" => 1024 * 1024 * 1024,
        "TB" => 1000 * 1000 * 1000 * 1000,
        "T" => 1024 * 1024 * 1024 * 1024,
        "PB" => 1000 * 1000 * 1000 * 1000 * 1000,
        "P" => 1024 * 1024 * 1024 * 1024 * 1024,
        "EB" => 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
        "E" => 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
        "ZB" => 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
        "Z" => 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
        "YB" => 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
        "Y" => 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
        _ => return Err("invalid byte multiple".to_string())
    };

    match from_str(num_str.as_slice()).and_then(|num: u64| Some(num * multiplier)) {
        Some(n) => Ok(n),
        None => fail!("BUG: failed to parse number. Please file a bug report with the command line.")
    }
}

pub fn to_human(bytes: u64) -> String {
    let mut s = String::new();

    let possible_suffixes = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    let mut i = 0;
    let mut human_bytes = bytes as f64;
    while human_bytes > 1000.0 && i < possible_suffixes.len() {
        i += 1;
        human_bytes /= 1000.0;
    }

    s = s.append(human_bytes.to_string().as_slice());
    s = s.append(" ");
    s = s.append(possible_suffixes[i]);

    s
}
