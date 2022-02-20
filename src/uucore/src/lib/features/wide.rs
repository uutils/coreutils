// This file is part of the uutils coreutils package.
//
// (c) Peter Atashian <retep998@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
pub trait ToWide {
    fn to_wide(&self) -> Vec<u16>;
    fn to_wide_null(&self) -> Vec<u16>;
}
impl<T> ToWide for T
where
    T: AsRef<OsStr>,
{
    fn to_wide(&self) -> Vec<u16> {
        self.as_ref().encode_wide().collect()
    }
    fn to_wide_null(&self) -> Vec<u16> {
        self.as_ref().encode_wide().chain(Some(0)).collect()
    }
}
pub trait FromWide {
    fn from_wide(wide: &[u16]) -> Self;
    fn from_wide_null(wide: &[u16]) -> Self;
}
impl FromWide for String {
    fn from_wide(wide: &[u16]) -> Self {
        OsString::from_wide(wide).to_string_lossy().into_owned()
    }
    fn from_wide_null(wide: &[u16]) -> Self {
        let len = wide.iter().take_while(|&&c| c != 0).count();
        OsString::from_wide(&wide[..len])
            .to_string_lossy()
            .into_owned()
    }
}
