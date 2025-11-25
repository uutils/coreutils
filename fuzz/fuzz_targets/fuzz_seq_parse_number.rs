#![no_main]

use libfuzzer_sys::fuzz_target;
use std::str::FromStr;
use uu_seq::number::PreciseNumber;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = PreciseNumber::from_str(s);
    }
});
