#![no_main]

use libfuzzer_sys::fuzz_target;
use uucore::parse_size::parse_size;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        _ = parse_size(s);
    }
});
