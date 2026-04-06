// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) strtime

//! Set of functions related to time handling

use jiff::Zoned;
use jiff::fmt::StdIoWrite;
use jiff::fmt::strtime::{BrokenDownTime, Config};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{UResult, USimpleError};
use crate::show_error;

#[cfg(feature = "i18n-datetime")]
pub use crate::i18n::datetime::NamePadding;

/// Controls whether locale name lookups return raw or padded names.
///
/// Without the `i18n-datetime` feature the parameter is accepted but ignored.
#[cfg(not(feature = "i18n-datetime"))]
#[derive(Clone, Copy)]
pub enum NamePadding {
    /// Raw names with no trailing padding — for `date` and similar utilities.
    Raw,
    /// Names padded to uniform display width — for columnar output like `ls`.
    Padded,
}

/// Format the given date according to this time format style.
fn format_zoned<W: Write>(out: &mut W, zoned: Zoned, fmt: &str) -> UResult<()> {
    let tm = BrokenDownTime::from(&zoned);
    let mut out = StdIoWrite(out);
    let config = Config::new().lenient(true);
    tm.format_with_config(&config, fmt, &mut out)
        .map_err(|x| USimpleError::new(1, x.to_string()))
}

/// Convert a SystemTime` to a number of seconds since UNIX_EPOCH
pub fn system_time_to_sec(time: SystemTime) -> (i64, u32) {
    if time > UNIX_EPOCH {
        let d = time.duration_since(UNIX_EPOCH).unwrap();
        (d.as_secs() as i64, d.subsec_nanos())
    } else {
        let d = UNIX_EPOCH.duration_since(time).unwrap();
        (-(d.as_secs() as i64), d.subsec_nanos())
    }
}

pub mod format {
    pub static FULL_ISO: &str = "%Y-%m-%d %H:%M:%S.%N %z";
    pub static LONG_ISO: &str = "%Y-%m-%d %H:%M";
    pub static ISO: &str = "%Y-%m-%d";
}

/// Sets how `format_system_time` behaves if the time cannot be converted.
pub enum FormatSystemTimeFallback {
    Integer,      // Just print seconds since epoch (`ls`)
    IntegerError, // The above, and print an error (`du``)
    Float,        // Just print seconds+nanoseconds since epoch (`stat`)
}

/// Write the seconds-since-epoch fallback used when a `SystemTime` is out of
/// the range representable by `jiff::Zoned`.
fn write_fallback_seconds<W: Write>(
    out: &mut W,
    time: SystemTime,
    mode: FormatSystemTimeFallback,
) -> UResult<()> {
    // TODO: The range allowed by jiff is different from what GNU accepts,
    // but it still far enough in the future/past to be unlikely to matter:
    //  jiff: Year between -9999 to 9999 (UTC) [-377705023201..=253402207200]
    //  GNU: Year fits in signed 32 bits (timezone dependent)
    let (mut secs, mut nsecs) = system_time_to_sec(time);
    match mode {
        FormatSystemTimeFallback::Integer => out.write_all(secs.to_string().as_bytes())?,
        FormatSystemTimeFallback::IntegerError => {
            let str = secs.to_string();
            show_error!("time '{str}' is out of range");
            out.write_all(str.as_bytes())?;
        }
        FormatSystemTimeFallback::Float => {
            if secs < 0 && nsecs != 0 {
                secs -= 1;
                nsecs = 1_000_000_000 - nsecs;
            }
            out.write_fmt(format_args!("{secs}.{nsecs:09}"))?;
        }
    }
    Ok(())
}

/// Format a `SystemTime` according to given fmt, and append to vector out.
pub fn format_system_time<W: Write>(
    out: &mut W,
    time: SystemTime,
    fmt: &str,
    mode: FormatSystemTimeFallback,
) -> UResult<()> {
    match time.try_into() {
        Ok(zoned) => format_zoned(out, zoned, fmt),
        // Assume that if we cannot build a Zoned element, the timestamp is
        // out of reasonable range, just print it then.
        Err(_) => write_fallback_seconds(out, time, mode),
    }
}

/// Like [`format_system_time`], but when built with the `i18n-datetime`
/// feature and a non-C `LC_TIME` locale is active, rewrites locale-dependent
/// strftime directives (`%b`, `%B`, `%a`, `%A`, and `%Y`/`%m`/`%d`/`%e` for
/// non-Gregorian calendars) to their localized values before formatting.
/// For Gregorian locales, `%Y`/`%m`/`%d`/`%e` are unaffected (e.g. `en_US`
/// still renders 2025 as `2025`).
///
/// With the feature disabled or a C/POSIX locale, this is identical to
/// `format_system_time`.
pub fn format_system_time_locale_aware<W: Write>(
    out: &mut W,
    time: SystemTime,
    fmt: &str,
    mode: FormatSystemTimeFallback,
    padding: NamePadding,
) -> UResult<()> {
    #[cfg(feature = "i18n-datetime")]
    {
        use crate::i18n::datetime::{localize_format_string, should_use_icu_locale};
        if should_use_icu_locale() {
            if let Ok(zoned) = <SystemTime as TryInto<Zoned>>::try_into(time) {
                let localized = localize_format_string(fmt, zoned.date(), padding);
                return format_zoned(out, zoned, &localized);
            }
            // Out-of-range: fall through to the plain fallback below.
        }
    }
    format_system_time(out, time, fmt, mode)
}

#[cfg(test)]
mod tests {
    use crate::time::{FormatSystemTimeFallback, format_system_time};
    use std::time::{Duration, UNIX_EPOCH};

    // Test epoch SystemTime get printed correctly at UTC0, with 2 simple formats.
    #[test]
    fn test_simple_system_time() {
        unsafe { std::env::set_var("TZ", "UTC0") };

        let time = UNIX_EPOCH;
        let mut out = Vec::new();
        format_system_time(
            &mut out,
            time,
            "%Y-%m-%d %H:%M",
            FormatSystemTimeFallback::Integer,
        )
        .expect("Formatting error.");
        assert_eq!(String::from_utf8(out).unwrap(), "1970-01-01 00:00");

        let mut out = Vec::new();
        format_system_time(
            &mut out,
            time,
            "%Y-%m-%d %H:%M:%S.%N %z",
            FormatSystemTimeFallback::Integer,
        )
        .expect("Formatting error.");
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "1970-01-01 00:00:00.000000000 +0000"
        );
    }

    // Test that very large (positive or negative) lead to just the timestamp being printed.
    #[test]
    fn test_large_system_time() {
        let time = UNIX_EPOCH + Duration::from_secs(67_768_036_191_763_200);
        let mut out = Vec::new();
        format_system_time(
            &mut out,
            time,
            "%Y-%m-%d %H:%M",
            FormatSystemTimeFallback::Integer,
        )
        .expect("Formatting error.");
        assert_eq!(String::from_utf8(out).unwrap(), "67768036191763200");

        let time = UNIX_EPOCH - Duration::from_secs(67_768_040_922_076_800);
        let mut out = Vec::new();
        format_system_time(
            &mut out,
            time,
            "%Y-%m-%d %H:%M",
            FormatSystemTimeFallback::Integer,
        )
        .expect("Formatting error.");
        assert_eq!(String::from_utf8(out).unwrap(), "-67768040922076800");
    }

    // Test that very large (positive or negative) lead to just the timestamp being printed.
    #[test]
    fn test_large_system_time_float() {
        let time =
            UNIX_EPOCH + Duration::from_secs(67_768_036_191_763_000) + Duration::from_nanos(123);
        let mut out = Vec::new();
        format_system_time(
            &mut out,
            time,
            "%Y-%m-%d %H:%M",
            FormatSystemTimeFallback::Float,
        )
        .expect("Formatting error.");
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "67768036191763000.000000123"
        );

        let time =
            UNIX_EPOCH - Duration::from_secs(67_768_040_922_076_000) + Duration::from_nanos(123);
        let mut out = Vec::new();
        format_system_time(
            &mut out,
            time,
            "%Y-%m-%d %H:%M",
            FormatSystemTimeFallback::Float,
        )
        .expect("Formatting error.");
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "-67768040922076000.000000123"
        );
    }
}
