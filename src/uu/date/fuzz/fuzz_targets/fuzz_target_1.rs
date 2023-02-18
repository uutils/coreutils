// spell-checker:ignore libfuzzer

#![no_main]
use libfuzzer_sys::fuzz_target;

use std::ffi::OsString;
use uu_date::uumain;

fuzz_target!(|data: &[u8]| {
    let iter: Vec<OsString> = [""].into_iter().map(|e| OsString::from(e)).collect();
    let it2 = iter.into_iter();
    uumain(it2);
});
