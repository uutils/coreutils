// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) strtime

//! Set of functions related to time handling

use jiff::Zoned;
use jiff::fmt::StdIoWrite;
use jiff::fmt::strtime::{BrokenDownTime, Config};
use std::time::{SystemTime, UNIX_EPOCH};

/// Format the given date according to this time format style.
fn format_zoned(out: &mut Vec<u8>, zoned: Zoned, fmt: &str) -> Result<(), jiff::Error> {
    let tm = BrokenDownTime::from(&zoned);
    let mut out = StdIoWrite(out);
    let config = Config::new().lenient(true);
    tm.format_with_config(&config, fmt, &mut out)
}

/// Format a `SystemTime` according to given fmt, and append to vector out.
pub fn format_system_time(
    out: &mut Vec<u8>,
    time: SystemTime,
    fmt: &str,
) -> Result<(), jiff::Error> {
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
            let ts = if time > UNIX_EPOCH {
                time.duration_since(UNIX_EPOCH).unwrap().as_secs()
            } else {
                out.extend(b"-"); // Add negative sign
                UNIX_EPOCH.duration_since(time).unwrap().as_secs()
            };
            out.extend(ts.to_string().as_bytes());
            Ok(())
        }
    }
}
