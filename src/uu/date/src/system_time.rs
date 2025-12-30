// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(unix)]
pub use unix::*;

#[cfg(unix)]
mod unix {
    use std::ffi::CString;
    use std::iter::Peekable;
    use std::str::Chars;

    use jiff::Zoned;
    use nix::libc;
    use uucore::error::{UResult, USimpleError};

    const COLON_Z_FORMATS: [&str; 3] = ["%:z", "%::z", "%:::z"];
    const COLON_LITERALS: [&str; 3] = ["%:", "%::", "%:::"];
    const HASH_Z_FORMATS: [&str; 1] = ["%z"];
    const HASH_LITERALS: [&str; 1] = ["%#"];
    const STRFTIME_BUF_LEN: usize = 1024;

    struct PrefixSpec<'a> {
        prefix: char,
        z_formats: &'a [&'a str],
        literal_formats: &'a [&'a str],
    }

    fn is_ethiopian_locale() -> bool {
        for var in ["LC_ALL", "LC_TIME", "LANG"] {
            if let Ok(val) = std::env::var(var) {
                if val.starts_with("am_ET") {
                    return true;
                }
            }
        }
        false
    }

    fn gregorian_to_ethiopian(y: i32, m: i32, d: i32) -> (i32, i32, i32) {
        let (m, y) = if m <= 2 { (m + 12, y - 1) } else { (m, y) };
        let jdn = (1461 * (y + 4800)) / 4
            + (367 * (m - 2)) / 12
            - (3 * ((y + 4900) / 100)) / 4
            + d
            - 32075;

        let n = jdn - 1724221;
        let n_cycle = n / 1461;
        let r = n % 1461;
        let y_rel = r / 365;
        let y_rel = if r == 1460 { 3 } else { y_rel };
        let year = 4 * n_cycle + y_rel + 1;
        let day_of_year = r - y_rel * 365;
        let month = day_of_year / 30 + 1;
        let day = day_of_year % 30 + 1;
        (year, month, day)
    }

    fn jiff_format(fmt: &str, date: &Zoned) -> UResult<String> {
        jiff::fmt::strtime::format(fmt, date)
            .map_err(|e| USimpleError::new(1, e.to_string()))
    }

    fn preprocess_format(format: &str, date: &Zoned) -> UResult<String> {
        let mut output = String::with_capacity(format.len());
        let mut chars = format.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '%' {
                let replacement = rewrite_directive(&mut chars, date)?;
                output.push_str(&replacement);
            } else {
                output.push(c);
            }
        }

        Ok(output)
    }

    fn rewrite_directive(chars: &mut Peekable<Chars<'_>>, date: &Zoned) -> UResult<String> {
        let Some(next) = chars.next() else {
            return Ok("%".to_string());
        };

        match next {
            'N' => {
                let nanos = date.timestamp().subsec_nanosecond();
                Ok(format!("{:09}", nanos))
            }
            's' => Ok(date.timestamp().as_second().to_string()),
            'q' => {
                let q = (date.month() - 1) / 3 + 1;
                Ok(q.to_string())
            }
            'z' => jiff_format("%z", date),
            '#' => rewrite_prefixed_z(
                chars,
                date,
                PrefixSpec {
                    prefix: '#',
                    z_formats: &HASH_Z_FORMATS,
                    literal_formats: &HASH_LITERALS,
                },
            ),
            ':' => rewrite_prefixed_z(
                chars,
                date,
                PrefixSpec {
                    prefix: ':',
                    z_formats: &COLON_Z_FORMATS,
                    literal_formats: &COLON_LITERALS,
                },
            ),
            '%' => Ok("%%".to_string()),
            _ => Ok(format!("%{next}")),
        }
    }

    fn rewrite_prefixed_z(
        chars: &mut Peekable<Chars<'_>>,
        date: &Zoned,
        spec: PrefixSpec<'_>,
    ) -> UResult<String> {
        let max_repeat = spec.z_formats.len();
        let extra = consume_repeats(chars, spec.prefix, max_repeat.saturating_sub(1));
        let count = 1 + extra;

        if matches!(chars.peek(), Some(&'z')) {
            chars.next();
            return jiff_format(spec.z_formats[count - 1], date);
        }

        Ok(spec.literal_formats[count - 1].to_string())
    }

    fn consume_repeats(
        chars: &mut Peekable<Chars<'_>>,
        needle: char,
        max: usize,
    ) -> usize {
        let mut count = 0;
        while count < max && matches!(chars.peek(), Some(ch) if *ch == needle) {
            chars.next();
            count += 1;
        }
        count
    }

    fn calendar_date(date: &Zoned) -> (i32, i32, i32) {
        if is_ethiopian_locale() {
            gregorian_to_ethiopian(date.year() as i32, date.month() as i32, date.day() as i32)
        } else {
            (date.year() as i32, date.month() as i32, date.day() as i32)
        }
    }

    fn build_tm(date: &Zoned) -> libc::tm {
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };

        tm.tm_sec = date.second() as i32;
        tm.tm_min = date.minute() as i32;
        tm.tm_hour = date.hour() as i32;

        let (year, month, day) = calendar_date(date);
        tm.tm_year = year - 1900;
        tm.tm_mon = month - 1;
        tm.tm_mday = day;

        tm.tm_wday = date.weekday().to_sunday_zero_offset() as i32;
        tm.tm_yday = date.day_of_year() as i32 - 1;
        tm.tm_isdst = -1;

        tm
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))]
    fn set_tm_zone(tm: &mut libc::tm, date: &Zoned) -> Option<CString> {
        tm.tm_gmtoff = date.offset().seconds() as _;

        let zone_cstring = jiff::fmt::strtime::format("%Z", date)
            .ok()
            .and_then(|abbrev| CString::new(abbrev).ok());
        if let Some(ref zone) = zone_cstring {
            tm.tm_zone = zone.as_ptr() as *mut i8;
        }
        zone_cstring
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    )))]
    fn set_tm_zone(_tm: &mut libc::tm, _date: &Zoned) -> Option<CString> {
        None
    }

    pub fn format_using_strftime(format: &str, date: &Zoned) -> UResult<String> {
        let format_string = preprocess_format(format, date)?;
        let mut tm = build_tm(date);
        let _zone_cstring = set_tm_zone(&mut tm, date);
        call_strftime(&format_string, &tm)
    }

    fn call_strftime(format_string: &str, tm: &libc::tm) -> UResult<String> {
        let format_c = CString::new(format_string)
            .map_err(|e| USimpleError::new(1, format!("Invalid format string: {e}")))?;

        let mut buffer = vec![0u8; STRFTIME_BUF_LEN];
        // SAFETY: `format_c` is NUL-terminated, `tm` is a valid libc::tm, and `buffer` is writable.
        let ret = unsafe {
            libc::strftime(
                buffer.as_mut_ptr() as *mut _,
                buffer.len(),
                format_c.as_ptr(),
                tm as *const _,
            )
        };

        if ret == 0 {
            return Err(USimpleError::new(1, "strftime failed or result too large"));
        }

        let len = ret as usize;
        Ok(String::from_utf8_lossy(&buffer[..len]).into_owned())
    }
}
