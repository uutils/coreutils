extern crate uucore;
use uucore::utmpx::c_utmp;

use std::ptr;

#[cfg(unix)]
extern "C" {
    fn getutxent() -> *const c_utmp;
    fn setutxent();
    fn endutxent();
}

pub struct UtmpIter;

impl UtmpIter {
    fn new() -> Self {
        unsafe {
            setutxent();
        }
        UtmpIter
    }
}

impl Iterator for UtmpIter {
    type Item = c_utmp;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let line = getutxent();

            if line.is_null() {
                endutxent();
                return None;
            }

            Some(ptr::read(line))
        }
    }
}

pub fn read_utmps() -> UtmpIter {
    UtmpIter::new()
}
