// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use libc::mkfifo;
use std::ffi::CString;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::show;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = crate::uu_app().try_get_matches_from(args)?;

    if matches.contains_id(crate::options::CONTEXT) {
        return Err(USimpleError::new(1, "--context is not implemented"));
    }
    if matches.get_flag(crate::options::SE_LINUX_SECURITY_CONTEXT) {
        return Err(USimpleError::new(1, "-Z is not implemented"));
    }

    let mode = match matches.get_one::<String>(crate::options::MODE) {
        Some(m) => match usize::from_str_radix(m, 8) {
            Ok(m) => m,
            Err(e) => return Err(USimpleError::new(1, format!("invalid mode: {e}"))),
        },
        None => 0o666,
    };

    let fifos: Vec<String> = match matches.get_many::<String>(crate::options::FIFO) {
        Some(v) => v.cloned().collect(),
        None => return Err(USimpleError::new(1, "missing operand")),
    };

    for f in fifos {
        let err = unsafe {
            let name = CString::new(f.as_bytes()).unwrap();
            mkfifo(name.as_ptr(), mode as libc::mode_t)
        };
        if err == -1 {
            show!(USimpleError::new(
                1,
                format!("cannot create fifo {}: File exists", f.quote())
            ));
        }
    }

    Ok(())
}
