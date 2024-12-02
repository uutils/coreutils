// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// This module contains classes and functions for dealing with the differences
// between operating systems regarding the lossless processing of OsStr/OsString.
// In contrast to existing crates with similar purpose, this module does not use any
// `unsafe` features or functions.
// Due to a suboptimal design aspect of OsStr/OsString on windows, we need to
// encode/decode to wide chars on windows operating system.
// This prevents borrowing from OsStr on windows. Anyway, if optimally used,#
// this conversion needs to be done only once in the beginning and at the end.

use std::ffi::OsString;
#[cfg(not(target_os = "windows"))]
use std::os::unix::ffi::{OsStrExt, OsStringExt};
#[cfg(target_os = "windows")]
use std::os::windows::prelude::*;
use std::{borrow::Cow, ffi::OsStr};

#[cfg(target_os = "windows")]
use u16 as NativeIntCharU;
#[cfg(not(target_os = "windows"))]
use u8 as NativeIntCharU;

pub type NativeCharInt = NativeIntCharU;
pub type NativeIntStr = [NativeCharInt];
pub type NativeIntString = Vec<NativeCharInt>;

pub struct NCvt;

pub trait Convert<From, To> {
    fn convert(f: From) -> To;
}

// ================ str/String =================

impl<'a> Convert<&'a str, Cow<'a, NativeIntStr>> for NCvt {
    fn convert(f: &'a str) -> Cow<'a, NativeIntStr> {
        #[cfg(target_os = "windows")]
        {
            Cow::Owned(f.encode_utf16().collect())
        }

        #[cfg(not(target_os = "windows"))]
        {
            Cow::Borrowed(f.as_bytes())
        }
    }
}

impl<'a> Convert<&'a String, Cow<'a, NativeIntStr>> for NCvt {
    fn convert(f: &'a String) -> Cow<'a, NativeIntStr> {
        #[cfg(target_os = "windows")]
        {
            Cow::Owned(f.encode_utf16().collect())
        }

        #[cfg(not(target_os = "windows"))]
        {
            Cow::Borrowed(f.as_bytes())
        }
    }
}

impl<'a> Convert<String, Cow<'a, NativeIntStr>> for NCvt {
    fn convert(f: String) -> Cow<'a, NativeIntStr> {
        #[cfg(target_os = "windows")]
        {
            Cow::Owned(f.encode_utf16().collect())
        }

        #[cfg(not(target_os = "windows"))]
        {
            Cow::Owned(f.into_bytes())
        }
    }
}

// ================ OsStr/OsString =================

impl<'a> Convert<&'a OsStr, Cow<'a, NativeIntStr>> for NCvt {
    fn convert(f: &'a OsStr) -> Cow<'a, NativeIntStr> {
        to_native_int_representation(f)
    }
}

impl<'a> Convert<&'a OsString, Cow<'a, NativeIntStr>> for NCvt {
    fn convert(f: &'a OsString) -> Cow<'a, NativeIntStr> {
        to_native_int_representation(f)
    }
}

impl<'a> Convert<OsString, Cow<'a, NativeIntStr>> for NCvt {
    fn convert(f: OsString) -> Cow<'a, NativeIntStr> {
        #[cfg(target_os = "windows")]
        {
            Cow::Owned(f.encode_wide().collect())
        }

        #[cfg(not(target_os = "windows"))]
        {
            Cow::Owned(f.into_vec())
        }
    }
}

// ================ Vec<Str/String> =================

impl<'a> Convert<&'a Vec<&'a str>, Vec<Cow<'a, NativeIntStr>>> for NCvt {
    fn convert(f: &'a Vec<&'a str>) -> Vec<Cow<'a, NativeIntStr>> {
        f.iter().map(|x| Self::convert(*x)).collect()
    }
}

impl<'a> Convert<Vec<&'a str>, Vec<Cow<'a, NativeIntStr>>> for NCvt {
    fn convert(f: Vec<&'a str>) -> Vec<Cow<'a, NativeIntStr>> {
        f.iter().map(|x| Self::convert(*x)).collect()
    }
}

impl<'a> Convert<&'a Vec<String>, Vec<Cow<'a, NativeIntStr>>> for NCvt {
    fn convert(f: &'a Vec<String>) -> Vec<Cow<'a, NativeIntStr>> {
        f.iter().map(Self::convert).collect()
    }
}

impl<'a> Convert<Vec<String>, Vec<Cow<'a, NativeIntStr>>> for NCvt {
    fn convert(f: Vec<String>) -> Vec<Cow<'a, NativeIntStr>> {
        f.into_iter().map(Self::convert).collect()
    }
}

pub fn to_native_int_representation(input: &OsStr) -> Cow<'_, NativeIntStr> {
    #[cfg(target_os = "windows")]
    {
        Cow::Owned(input.encode_wide().collect())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Cow::Borrowed(input.as_bytes())
    }
}

#[allow(clippy::needless_pass_by_value)] // needed on windows
pub fn from_native_int_representation(input: Cow<'_, NativeIntStr>) -> Cow<'_, OsStr> {
    #[cfg(target_os = "windows")]
    {
        Cow::Owned(OsString::from_wide(&input))
    }

    #[cfg(not(target_os = "windows"))]
    {
        match input {
            Cow::Borrowed(borrow) => Cow::Borrowed(OsStr::from_bytes(borrow)),
            Cow::Owned(own) => Cow::Owned(OsString::from_vec(own)),
        }
    }
}

#[allow(clippy::needless_pass_by_value)] // needed on windows
pub fn from_native_int_representation_owned(input: NativeIntString) -> OsString {
    #[cfg(target_os = "windows")]
    {
        OsString::from_wide(&input)
    }

    #[cfg(not(target_os = "windows"))]
    {
        OsString::from_vec(input)
    }
}

pub fn get_single_native_int_value(c: &char) -> Option<NativeCharInt> {
    #[cfg(target_os = "windows")]
    {
        let mut buf = [0u16, 0];
        let s = c.encode_utf16(&mut buf);
        if s.len() == 1 {
            Some(buf[0])
        } else {
            None
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let mut buf = [0u8, 0, 0, 0];
        let s = c.encode_utf8(&mut buf);
        if s.len() == 1 {
            Some(buf[0])
        } else {
            None
        }
    }
}

pub fn get_char_from_native_int(ni: NativeCharInt) -> Option<(char, NativeCharInt)> {
    let c_opt;
    #[cfg(target_os = "windows")]
    {
        c_opt = char::decode_utf16([ni; 1]).next().unwrap().ok();
    };

    #[cfg(not(target_os = "windows"))]
    {
        c_opt = std::str::from_utf8(&[ni; 1])
            .ok()
            .map(|x| x.chars().next().unwrap());
    };

    if let Some(c) = c_opt {
        return Some((c, ni));
    }

    None
}

pub struct NativeStr<'a> {
    native: Cow<'a, NativeIntStr>,
}

impl<'a> NativeStr<'a> {
    pub fn new(str: &'a OsStr) -> Self {
        Self {
            native: to_native_int_representation(str),
        }
    }

    pub fn native(&self) -> Cow<'a, NativeIntStr> {
        self.native.clone()
    }

    pub fn into_native(self) -> Cow<'a, NativeIntStr> {
        self.native
    }

    pub fn contains(&self, x: &char) -> Option<bool> {
        let n_c = get_single_native_int_value(x)?;
        Some(self.native.contains(&n_c))
    }

    pub fn slice(&self, from: usize, to: usize) -> Cow<'a, OsStr> {
        let result = self.match_cow(|b| Ok::<_, ()>(&b[from..to]), |o| Ok(o[from..to].to_vec()));
        result.unwrap()
    }

    pub fn split_once(&self, pred: &char) -> Option<(Cow<'a, OsStr>, Cow<'a, OsStr>)> {
        let n_c = get_single_native_int_value(pred)?;
        let p = self.native.iter().position(|&x| x == n_c)?;
        let before = self.slice(0, p);
        let after = self.slice(p + 1, self.native.len());
        Some((before, after))
    }

    pub fn split_at(&self, pos: usize) -> (Cow<'a, OsStr>, Cow<'a, OsStr>) {
        let before = self.slice(0, pos);
        let after = self.slice(pos, self.native.len());
        (before, after)
    }

    pub fn strip_prefix(&self, prefix: &OsStr) -> Option<Cow<'a, OsStr>> {
        let n_prefix = to_native_int_representation(prefix);
        let result = self.match_cow(
            |b| b.strip_prefix(&*n_prefix).ok_or(()),
            |o| o.strip_prefix(&*n_prefix).map(|x| x.to_vec()).ok_or(()),
        );
        result.ok()
    }

    pub fn strip_prefix_native(&self, prefix: &OsStr) -> Option<Cow<'a, NativeIntStr>> {
        let n_prefix = to_native_int_representation(prefix);
        let result = self.match_cow_native(
            |b| b.strip_prefix(&*n_prefix).ok_or(()),
            |o| o.strip_prefix(&*n_prefix).map(|x| x.to_vec()).ok_or(()),
        );
        result.ok()
    }

    fn match_cow<FnBorrow, FnOwned, Err>(
        &self,
        f_borrow: FnBorrow,
        f_owned: FnOwned,
    ) -> Result<Cow<'a, OsStr>, Err>
    where
        FnBorrow: FnOnce(&'a [NativeCharInt]) -> Result<&'a [NativeCharInt], Err>,
        FnOwned: FnOnce(&Vec<NativeCharInt>) -> Result<Vec<NativeCharInt>, Err>,
    {
        match &self.native {
            Cow::Borrowed(b) => {
                let slice = f_borrow(b);
                let os_str = slice.map(|x| from_native_int_representation(Cow::Borrowed(x)));
                os_str
            }
            Cow::Owned(o) => {
                let slice = f_owned(o);
                let os_str = slice.map(from_native_int_representation_owned);
                os_str.map(Cow::Owned)
            }
        }
    }

    fn match_cow_native<FnBorrow, FnOwned, Err>(
        &self,
        f_borrow: FnBorrow,
        f_owned: FnOwned,
    ) -> Result<Cow<'a, NativeIntStr>, Err>
    where
        FnBorrow: FnOnce(&'a [NativeCharInt]) -> Result<&'a [NativeCharInt], Err>,
        FnOwned: FnOnce(&Vec<NativeCharInt>) -> Result<Vec<NativeCharInt>, Err>,
    {
        match &self.native {
            Cow::Borrowed(b) => {
                let slice = f_borrow(b);
                slice.map(Cow::Borrowed)
            }
            Cow::Owned(o) => {
                let slice = f_owned(o);
                slice.map(Cow::Owned)
            }
        }
    }
}
