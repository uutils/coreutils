// # Read, Write, BufRead, Seek
//use std::io::prelude::*;

//use std::io::Result as IOResult;

//use std::borrow::Cow;
//use std::borrow::Borrow;
//use std::convert::AsRef;

extern crate libc;

extern crate uucore;
use uucore::utmpx;
use uucore::utmpx::c_utmp;

use std::slice;
use std::io::Read;
use std::fs::File;
use std::path::Path;
use std::convert::AsRef;
use std::io::Result as IOResult;
use std::marker::PhantomData;
use std::mem;

pub struct StIter<T> {
    f: File,
    size: usize,
    _p: PhantomData<T>,
}

impl<T> Iterator for StIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let mut s = mem::zeroed();
            let mut buf = slice::from_raw_parts_mut(&mut s as *mut Self::Item as *mut u8, self.size);
            if let Ok(()) = self.f.read_exact(buf) {
                Some(s)
            } else {
                mem::forget(s);
                None
            }
        }
    }
}

fn read_structs<T, P: AsRef<Path>>(p: P) -> IOResult<StIter<T>> {
    Ok(StIter {
        f: try!(File::open(p)),
        size: mem::size_of::<T>(),
        _p: PhantomData,
    })
}

pub fn read_utmps() -> IOResult<StIter<c_utmp>> {
    read_structs(utmpx::DEFAULT_FILE)
}
