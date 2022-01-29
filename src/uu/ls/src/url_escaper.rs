use std::{
    borrow::Cow,
    convert::{TryFrom, TryInto},
    ffi::OsStr,
};

use ascii::{AsciiChar, AsciiStr, AsciiString};
/// this is logically enum ByteOrResult {Char(AsciiChar), Escape([AsciiChar; 2])}
/// but we don't use that definition because rustc uses 3 bytes instead of 2 for it (even if we use a pair instead of the array)
#[derive(Copy, Clone)]
#[repr(C)]
struct AsciiOrEscape(Option<AsciiChar>, AsciiChar);

impl AsciiOrEscape {
    pub fn ascii(ch: AsciiChar) -> Self {
        Self(None, ch)
    }

    pub fn escape(c0: AsciiChar, c1: AsciiChar) -> Self {
        Self(Some(c0), c1)
    }

    #[inline]
    pub fn classify(self) -> Result<AsciiChar, [AsciiChar; 2]> {
        if let Some(c0) = self.0 {
            Err([c0, self.1])
        } else {
            Ok(self.1)
        }
    }

    pub fn is_byte(self) -> bool {
        self.0.is_none()
    }
}

pub struct UrlEscaper {
    table: Box<[AsciiOrEscape; 256]>,
}

impl UrlEscaper {
    pub fn new() -> Self {
        let mut table = Vec::with_capacity(256);
        static HEX_BYTES: &[u8; 16] = b"0123456789abcdef";
        let hex_bytes = AsciiStr::from_ascii(HEX_BYTES).unwrap();
        for &c0 in hex_bytes {
            for &c1 in hex_bytes {
                table.push(AsciiOrEscape::escape(c0, c1));
            }
        }
        assert_eq!(table.len(), 256);
        for r in [b'0'..=b'9', b'A'..=b'Z', b'a'..=b'z'] {
            for b in r {
                table[b as usize] = AsciiOrEscape::ascii(AsciiChar::from_ascii(b).unwrap());
            }
        }
        for b in [b'_', b'-', b'.', b'~'] {
            table[b as usize] = AsciiOrEscape::ascii(AsciiChar::from_ascii(b).unwrap());
        }
        Self {
            table: table.into_boxed_slice().try_into().map_err(|_| ()).unwrap(),
        }
    }

    pub fn into_path_escaper(mut self) -> Self {
        let slash = AsciiOrEscape::ascii(AsciiChar::from_ascii(b'/').unwrap());
        self.table[b'/' as usize] = slash;
        if std::path::MAIN_SEPARATOR != '/' {
            if let Some(main_separator) = self.table.get_mut(std::path::MAIN_SEPARATOR as usize) {
                *main_separator = slash;
            }
        }
        self
    }

    pub fn escape<'a>(&self, s: &'a [u8]) -> Cow<'a, AsciiStr> {
        let table = &*self.table;

        let mut not_escapes = 0usize;
        let mut not_identities = false;
        for b in s.iter().copied() {
            if std::path::MAIN_SEPARATOR != '/' && (std::path::MAIN_SEPARATOR as usize) < 256 {
                not_identities |= b == (std::path::MAIN_SEPARATOR as u8);
            }
            // cannot overflow because it will be at most s.len() since we are summing 0 or 1 values
            not_escapes = not_escapes.wrapping_add(table[b as usize].is_byte() as usize);
        }

        if not_escapes == s.len() && !not_identities {
            // SAFETY: this is safe because the string only has ASCII characters (otherwise it would need escapes)
            Cow::Borrowed(unsafe { AsciiStr::from_ascii_unchecked(s) })
        } else {
            static ERROR: &str = "escaped string length too large to fit in memory";
            // Vec only supports capacity up to isize::MAX, so we must use isize to do all checks
            let len = isize::try_from(s.len()).expect(ERROR);
            // can't wrap because not_escapes <= s.len
            let escapes = len.wrapping_sub(not_escapes as isize);
            // must check for overflow since we are using unsafe code
            let size = len
                .checked_add(escapes)
                .expect(ERROR)
                .checked_add(escapes)
                .expect(ERROR);

            let mut res = Vec::with_capacity(size as usize);
            let mut out: *mut AsciiChar = res.as_mut_ptr();

            for b in s.iter().copied() {
                match table[b as usize].classify() {
                    Ok(ch) => {
                        // SAFETY: we precompute the correct size
                        unsafe {
                            out.write(ch);
                            out = out.offset(1);
                        }
                    }
                    Err(escape) => {
                        // SAFETY: we precompute the correct size
                        unsafe {
                            out.write(AsciiChar::Percent);
                            out.offset(1).write(escape[0]);
                            out.offset(2).write(escape[1]);
                            out = out.offset(3);
                        }
                    }
                }
            }
            debug_assert_eq!(unsafe { out.offset_from(res.as_mut_ptr()) }, size);
            unsafe { res.set_len(size as usize) };

            Cow::Owned(AsciiString::from(res))
        }
    }

    #[cfg(unix)]
    pub fn escape_os<'a>(&self, s: &'a OsStr) -> Option<Cow<'a, AsciiStr>> {
        use std::os::unix::prelude::OsStrExt;
        Some(self.escape(s.as_bytes()))
    }

    #[cfg(not(unix))]
    pub fn escape_os<'a>(&self, s: &'a OsStr) -> Option<Cow<'a, AsciiStr>> {
        s.to_str().map(|x| self.escape(x.as_bytes()))
    }
}
