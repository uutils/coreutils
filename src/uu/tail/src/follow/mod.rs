// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

mod files;
#[cfg(not(target_os = "wasi"))]
mod watch;

#[cfg(not(target_os = "wasi"))]
pub use watch::{Observer, follow};

// WASI: notify/inotify are unavailable, so `tail -f` cannot work.
// Provide minimal stubs matching the real Observer API so tail compiles.
#[cfg(target_os = "wasi")]
mod wasi_stubs {
    use crate::args::Settings;
    use std::io::BufRead;
    use std::path::Path;
    use uucore::error::{UResult, USimpleError};

    pub struct Observer {
        pub use_polling: bool,
        pub pid: super::super::platform::Pid,
    }

    impl Observer {
        pub fn from(settings: &Settings) -> Self {
            Self {
                use_polling: false,
                pid: settings.pid,
            }
        }

        pub fn start(&mut self, _settings: &Settings) -> UResult<()> {
            Ok(())
        }

        pub fn add_path(
            &mut self,
            _path: &Path,
            _display_name: &str,
            _reader: Option<Box<dyn BufRead>>,
            _update_last: bool,
        ) -> UResult<()> {
            Ok(())
        }

        pub fn add_bad_path(
            &mut self,
            _path: &Path,
            _display_name: &str,
            _update_last: bool,
        ) -> UResult<()> {
            Ok(())
        }

        pub fn follow_name_retry(&self) -> bool {
            false
        }
    }

    pub fn follow(_observer: Observer, _settings: &Settings) -> UResult<()> {
        Err(USimpleError::new(
            1,
            "follow mode is not supported on this platform",
        ))
    }
}

#[cfg(target_os = "wasi")]
pub use wasi_stubs::{Observer, follow};
