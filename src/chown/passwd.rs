// (c) Jian Zeng <Anonymousknight96@gmail.com>

extern crate uucore;
use self::uucore::c_types::{getpwuid, getpwnam, getgrgid, getgrnam};

use std::ptr;
use std::ffi::{CString, CStr};
use std::io::Result as IOResult;
use std::io::{ErrorKind, Error};

macro_rules! gen_func {
    ($fun:ident, $getid:ident, $getnm:ident, $field:ident) => (
        pub fn $fun(name_or_id: &str) -> IOResult<u32> {
            if let Ok(id) = name_or_id.parse::<u32>() {
                let data = unsafe {
                    $getid(id)
                };
                if !data.is_null() {
                    return Ok(id);
                } else {
                    return Err(Error::new(ErrorKind::NotFound, format!("No such id `{}`", id)));
                }
            } else {
                let name = CString::new(name_or_id).unwrap();
                let data = unsafe {
                    $getnm(name.as_ptr())
                };
                if !data.is_null() {
                    return Ok(unsafe {
                        ptr::read(data).$field
                    });
                } else {
                    return Err(Error::new(ErrorKind::NotFound, format!("No such name `{}`", name_or_id)));
                }
            }
        }
    );
    ($fun:ident, $getid:ident, $field:ident) => (
        pub fn $fun(id: u32) -> IOResult<String> {
            let data = unsafe {
                $getid(id)
            };
            if !data.is_null() {
                Ok(unsafe {
                    CStr::from_ptr(ptr::read(data).$field).to_string_lossy().into_owned()
                })
            } else {
                Err(Error::new(ErrorKind::NotFound, format!("No such id `{}`", id)))
            }
        }
    );
}

gen_func!(getuid, getpwuid, getpwnam, pw_uid);
gen_func!(getgid, getgrgid, getgrnam, gr_gid);
gen_func!(uid2usr, getpwuid, pw_name);
gen_func!(gid2grp, getgrgid, gr_name);
