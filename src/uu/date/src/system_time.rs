// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(unix)]
pub use unix::*;

#[cfg(unix)]
mod unix {
    use std::ffi::{CString, CStr};
    use jiff::Zoned;
    use uucore::error::{UResult, USimpleError};
    use nix::libc;

    pub fn format_using_strftime(format: &str, date: &Zoned) -> UResult<String> {
        // Preprocess format string to handle extensions not supported by standard strftime
        // or where we want to ensure specific behavior (like %N).
        // specific specifiers: %N, %q, %:z, %::z, %:::z
        // We use jiff to format these specific parts.
        
        let mut new_fmt = String::with_capacity(format.len());
        let mut chars = format.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c == '%' {
                if let Some(&next) = chars.peek() {
                    match next {
                        'N' => { 
                             chars.next();
                             let nanos = date.timestamp().subsec_nanosecond();
                             // eprintln!("DEBUG: Nanos: {}, Full Date: {:?}", nanos, date);
                             let s = format!("{:09}", nanos);
                             new_fmt.push_str(&s);
                        }
                        's' => {
                             chars.next();
                             let s = date.timestamp().as_second().to_string();
                             new_fmt.push_str(&s);
                        }
                        'q' => {
                             chars.next();
                             let q = (date.month() - 1) / 3 + 1;
                             new_fmt.push_str(&q.to_string());
                        }
                        'z' => {
                             chars.next();
                             // %z -> +hhmm
                             // jiff %z matches this
                             new_fmt.push_str(&jiff::fmt::strtime::format("%z", date).map_err(|e| USimpleError::new(1, e.to_string()))?);
                        }
                        '#' => {
                             chars.next(); // eat #
                             if let Some(&n2) = chars.peek() {
                                 if n2 == 'z' {
                                     chars.next();
                                     // %#z -> treated as %z
                                     new_fmt.push_str(&jiff::fmt::strtime::format("%z", date).map_err(|e| USimpleError::new(1, e.to_string()))?);
                                 } else {
                                     new_fmt.push_str("%#");
                                 }
                             } else {
                                 new_fmt.push_str("%#");
                             }
                        }
                        ':' => {
                            // Check for :z, ::z, :::z
                             chars.next(); // eat :
                             if let Some(&n2) = chars.peek() {
                                 if n2 == 'z' {
                                     chars.next();
                                     // %:z
                                     new_fmt.push_str(&jiff::fmt::strtime::format("%:z", date).map_err(|e| USimpleError::new(1, e.to_string()))?);
                                 } else if n2 == ':' {
                                     chars.next();
                                     if let Some(&n3) = chars.peek() {
                                         if n3 == 'z' {
                                              chars.next();
                                              // %::z
                                              new_fmt.push_str(&jiff::fmt::strtime::format("%::z", date).map_err(|e| USimpleError::new(1, e.to_string()))?);
                                         } else if n3 == ':' {
                                             chars.next();
                                             if let Some(&n4) = chars.peek() {
                                                 if n4 == 'z' {
                                                     chars.next();
                                                     // %:::z
                                                     new_fmt.push_str(&jiff::fmt::strtime::format("%:::z", date).map_err(|e| USimpleError::new(1, e.to_string()))?);
                                                 } else {
                                                     new_fmt.push_str("%:::");
                                                 }
                                             } else {
                                                 new_fmt.push_str("%:::");
                                             }
                                         } else {
                                             new_fmt.push_str("%::");
                                         }
                                     } else {
                                          new_fmt.push_str("%::");
                                     }
                                 } else {
                                     new_fmt.push_str("%:");
                                 }
                             } else {
                                 new_fmt.push_str("%:");
                             }
                        }
                        // Handle standard escape %%
                        '%' => {
                             chars.next();
                             new_fmt.push_str("%%");
                        }
                        _ => {
                            new_fmt.push('%');
                            // Let strftime handle the next char, just loop around
                        }
                    }
                } else {
                    new_fmt.push('%');
                }
            } else {
                new_fmt.push(c);
            }
        }
        
        let format_string = new_fmt;

        // Convert jiff::Zoned to libc::tm
        // Use mem::zeroed to handle platform differences in struct fields
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        
        tm.tm_sec = date.second() as i32;
        tm.tm_min = date.minute() as i32;
        tm.tm_hour = date.hour() as i32;
        tm.tm_mday = date.day() as i32;
        tm.tm_mon = date.month() as i32 - 1; // tm_mon is 0-11
        tm.tm_year = date.year() as i32 - 1900; // tm_year is years since 1900
        tm.tm_wday = date.weekday().to_sunday_zero_offset() as i32;
        tm.tm_yday = date.day_of_year() as i32 - 1; // tm_yday is 0-365
        tm.tm_isdst = -1; // Let libraries determine if needed, though for formatting typically unused/ignored or uses global if zone not set

        // We need to keep the CString for tm_zone valid during strftime usage
        // So we declare it here
        let zone_cstring;

        // Set timezone fields on supported platforms
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd", target_os = "dragonfly"))]
        {
            tm.tm_gmtoff = date.offset().seconds() as _;
            
            // Populate tm_zone
            // We can get the abbreviation from date.time_zone().
            // Note: date.time_zone() returns a TimeZone, we need the abbreviation for the specific instant?
            // date.datetime() returns civil time.
            // jiff::Zoned has `time_zone()` and `offset()`.
            // The abbreviation usually depends on whether DST is active.
            // checking `date` (Zoned) string representation usually includes it?
            // `jiff` doesn't seem to expose `abbreviation()` directly on Zoned nicely?
            // Wait, standard `strftime` (%Z) looks at `tm_zone`.
            // How do we get abbreviation from jiff::Zoned?
            // `date.time_zone()` is the TZDB entry.
            // `date.offset()` is the offset.
            // We can try to format with %Z using jiff and use that string?
            if let Ok(abbrev_string) = jiff::fmt::strtime::format("%Z", date) {
                 zone_cstring = CString::new(abbrev_string).ok();
                 if let Some(ref nz) = zone_cstring {
                     tm.tm_zone = nz.as_ptr() as *mut i8;
                 }
            } else {
                 zone_cstring = None;
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd", target_os = "dragonfly")))]
        {
            zone_cstring = None;
        }

        let format_c = CString::new(format_string).map_err(|e| {
             USimpleError::new(1, format!("Invalid format string: {}", e))
        })?;

        let mut buffer = vec![0u8; 1024];
        let ret = unsafe {
            libc::strftime(
                buffer.as_mut_ptr() as *mut _,
                buffer.len(),
                format_c.as_ptr(),
                &tm as *const _
            )
        };

        if ret == 0 {
             return Err(USimpleError::new(1, "strftime failed or result too large"));
        }

        let c_str = unsafe { CStr::from_ptr(buffer.as_ptr() as *const _) };
        let s = c_str.to_string_lossy().into_owned();
        Ok(s)
    }
}
