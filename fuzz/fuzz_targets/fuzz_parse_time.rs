// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#![no_main]

use libfuzzer_sys::fuzz_target;
use uucore::parser::parse_time;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        _ = parse_time::from_str(s, true);
        _ = parse_time::from_str(s, false);
    }
});
