#[macro_use]
mod common;

#[allow(unused_imports)]
#[cfg(unix)]
#[macro_use]
extern crate lazy_static;

#[cfg(unix)]
extern crate rust_users;

include!(concat!(env!("OUT_DIR"), "/test_modules.rs"));
