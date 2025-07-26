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

/// Format the given date according to this time format style.
fn format_zoned<W: Write>(out: &mut W, zoned: Zoned, fmt: &str) -> UResult<()> {
    let tm = BrokenDownTime::from(&zoned);
    let mut out = StdIoWrite(out);
    let config = Config::new().lenient(true);
    tm.format_with_config(&config, fmt, &mut out)
        .map_err(|x| USimpleError::new(1, x.to_string()))
}

/// Format a `SystemTime` according to given fmt, and append to vector out.
pub fn format_system_time<W: Write>(
    out: &mut W,
    time: SystemTime,
    fmt: &str,
    show_error: bool,
) -> UResult<()> {
    let zoned: Result<Zoned, _> = time.try_into();
    match zoned {
        Ok(zoned) => format_zoned(out, zoned, fmt),
        Err(_) => {
            // Assume that if we cannot build a Zoned element, the timestamp is
            // out of reasonable range, just print it then.
            // TODO: The range allowed by jiff is different from what GNU accepts,
            // but it still far enough in the future/past to be unlikely to matter:
            //  jiff: Year between -9999 to 9999 (UTC) [-377705023201..=253402207200]
            //  GNU: Year fits in signed 32 bits (timezone dependent)
            let ts: i64 = if time > UNIX_EPOCH {
                time.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
            } else {
                -(UNIX_EPOCH.duration_since(time).unwrap().as_secs() as i64)
            };
            let str = ts.to_string();
            if show_error {
                show_error!("time '{str}' is out of range");
            }
            out.write_all(str.as_bytes())?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::time::format_system_time;
    use std::time::{Duration, UNIX_EPOCH};

    // Test epoch SystemTime get printed correctly at UTC0, with 2 simple formats.
    #[test]
    fn test_simple_system_time() {
        unsafe { std::env::set_var("TZ", "UTC0") };

        let time = UNIX_EPOCH;
        let mut out = Vec::new();
        format_system_time(&mut out, time, "%Y-%m-%d %H:%M", false).expect("Formatting error.");
        assert_eq!(String::from_utf8(out).unwrap(), "1970-01-01 00:00");

        let mut out = Vec::new();
        format_system_time(&mut out, time, "%Y-%m-%d %H:%M:%S.%N %z", false)
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
        format_system_time(&mut out, time, "%Y-%m-%d %H:%M", false).expect("Formatting error.");
        assert_eq!(String::from_utf8(out).unwrap(), "67768036191763200");

        let time = UNIX_EPOCH - Duration::from_secs(67_768_040_922_076_800);
        let mut out = Vec::new();
        format_system_time(&mut out, time, "%Y-%m-%d %H:%M", false).expect("Formatting error.");
        assert_eq!(String::from_utf8(out).unwrap(), "-67768040922076800");
    }
}
