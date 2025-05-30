// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

/* Last synced with: sync (GNU coreutils) 8.13 */

use clap::{Arg, ArgAction, Command};
#[cfg(any(target_os = "linux", target_os = "android"))]
use nix::errno::Errno;
#[cfg(any(target_os = "linux", target_os = "android"))]
use nix::fcntl::{OFlag, open};
#[cfg(any(target_os = "linux", target_os = "android"))]
use nix::sys::stat::Mode;
use std::path::Path;
use uucore::display::Quotable;
#[cfg(any(target_os = "linux", target_os = "android"))]
use uucore::error::FromIo;
use uucore::error::{UResult, USimpleError};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("sync.md");
const USAGE: &str = help_usage!("sync.md");

pub mod options {
    pub static FILE_SYSTEM: &str = "file-system";
    pub static DATA: &str = "data";
}

static ARG_FILES: &str = "files";

#[cfg(unix)]
mod platform {
    use nix::unistd::sync;
    #[cfg(any(target_os = "linux", target_os = "android"))]
    use nix::unistd::{fdatasync, syncfs};
    #[cfg(any(target_os = "linux", target_os = "android"))]
    use std::fs::File;
    use uucore::error::UResult;

    pub fn do_sync() -> UResult<()> {
        sync();
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn do_syncfs(files: Vec<String>) -> UResult<()> {
        for path in files {
            let f = File::open(path).unwrap();
            syncfs(f)?;
        }
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn do_fdatasync(files: Vec<String>) -> UResult<()> {
        for path in files {
            let f = File::open(path).unwrap();
            fdatasync(f)?;
        }
        Ok(())
    }
}

#[cfg(windows)]
mod platform {
    use std::fs::OpenOptions;
    use std::os::windows::prelude::*;
    use std::path::Path;
    use uucore::error::{UResult, USimpleError};
    use uucore::wide::{FromWide, ToWide};
    use windows_sys::Win32::Foundation::{
        ERROR_NO_MORE_FILES, GetLastError, HANDLE, INVALID_HANDLE_VALUE, MAX_PATH,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, FlushFileBuffers, GetDriveTypeW,
    };
    use windows_sys::Win32::System::WindowsProgramming::DRIVE_FIXED;

    fn get_last_error() -> u32 {
        // SAFETY: `GetLastError` has no safety preconditions
        unsafe { GetLastError() as u32 }
    }

    fn flush_volume(name: &str) -> UResult<()> {
        let name_wide = name.to_wide_null();
        // SAFETY: `name` is a valid `str`, so `name_wide` is valid null-terminated UTF-16
        if unsafe { GetDriveTypeW(name_wide.as_ptr()) } == DRIVE_FIXED {
            let sliced_name = &name[..name.len() - 1]; // eliminate trailing backslash
            match OpenOptions::new().write(true).open(sliced_name) {
                Ok(file) => {
                    // SAFETY: `file` is a valid `File`
                    if unsafe { FlushFileBuffers(file.as_raw_handle() as HANDLE) } == 0 {
                        Err(USimpleError::new(
                            get_last_error() as i32,
                            "failed to flush file buffer",
                        ))
                    } else {
                        Ok(())
                    }
                }
                Err(e) => Err(USimpleError::new(
                    e.raw_os_error().unwrap_or(1),
                    "failed to create volume handle",
                )),
            }
        } else {
            Ok(())
        }
    }

    fn find_first_volume() -> UResult<(String, HANDLE)> {
        let mut name: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
        // SAFETY: `name` was just constructed and in scope, `len()` is its length by definition
        let handle = unsafe { FindFirstVolumeW(name.as_mut_ptr(), name.len() as u32) };
        if handle == INVALID_HANDLE_VALUE {
            return Err(USimpleError::new(
                get_last_error() as i32,
                "failed to find first volume",
            ));
        }
        Ok((String::from_wide_null(&name), handle))
    }

    fn find_all_volumes() -> UResult<Vec<String>> {
        let (first_volume, next_volume_handle) = find_first_volume()?;
        let mut volumes = vec![first_volume];
        loop {
            let mut name: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
            // SAFETY: `next_volume_handle` was returned by `find_first_volume`,
            // `name` was just constructed and in scope, `len()` is its length by definition
            if unsafe { FindNextVolumeW(next_volume_handle, name.as_mut_ptr(), name.len() as u32) }
                == 0
            {
                return match get_last_error() {
                    ERROR_NO_MORE_FILES => {
                        // SAFETY: `next_volume_handle` was returned by `find_first_volume`
                        unsafe { FindVolumeClose(next_volume_handle) };
                        Ok(volumes)
                    }
                    err => Err(USimpleError::new(err as i32, "failed to find next volume")),
                };
            } else {
                volumes.push(String::from_wide_null(&name));
            }
        }
    }

    pub fn do_sync() -> UResult<()> {
        let volumes = find_all_volumes()?;
        for vol in &volumes {
            flush_volume(vol)?;
        }
        Ok(())
    }

    pub fn do_syncfs(files: Vec<String>) -> UResult<()> {
        for path in files {
            flush_volume(
                Path::new(&path)
                    .components()
                    .next()
                    .unwrap()
                    .as_os_str()
                    .to_str()
                    .unwrap(),
            )?;
        }
        Ok(())
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let files: Vec<String> = matches
        .get_many::<String>(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    if matches.get_flag(options::DATA) && files.is_empty() {
        return Err(USimpleError::new(1, "--data needs at least one argument"));
    }

    for f in &files {
        // Use the Nix open to be able to set the NONBLOCK flags for fifo files
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            let path = Path::new(&f);
            if let Err(e) = open(path, OFlag::O_NONBLOCK, Mode::empty()) {
                if e != Errno::EACCES || (e == Errno::EACCES && path.is_dir()) {
                    e.map_err_context(|| format!("error opening {}", f.quote()))?;
                }
            }
        }

        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        {
            if !Path::new(&f).exists() {
                return Err(USimpleError::new(
                    1,
                    format!("error opening {}: No such file or directory", f.quote()),
                ));
            }
        }
    }

    #[allow(clippy::if_same_then_else)]
    if matches.get_flag(options::FILE_SYSTEM) {
        #[cfg(any(target_os = "linux", target_os = "android", target_os = "windows"))]
        syncfs(files)?;
    } else if matches.get_flag(options::DATA) {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        fdatasync(files)?;
    } else {
        sync()?;
    }
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE_SYSTEM)
                .short('f')
                .long(options::FILE_SYSTEM)
                .conflicts_with(options::DATA)
                .help("sync the file systems that contain the files (Linux and Windows only)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DATA)
                .short('d')
                .long(options::DATA)
                .conflicts_with(options::FILE_SYSTEM)
                .help("sync only file data, no unneeded metadata (Linux only)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath),
        )
}

fn sync() -> UResult<()> {
    platform::do_sync()
}

#[cfg(any(target_os = "linux", target_os = "android", target_os = "windows"))]
fn syncfs(files: Vec<String>) -> UResult<()> {
    platform::do_syncfs(files)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn fdatasync(files: Vec<String>) -> UResult<()> {
    platform::do_fdatasync(files)
}
