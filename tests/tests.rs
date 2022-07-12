#[macro_use]
mod common;

#[cfg(unix)]
extern crate rust_users;

include!(concat!(env!("OUT_DIR"), "/test_modules.rs"));
