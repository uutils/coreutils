// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use std::path::PathBuf;
use uucore::error::{UResult, USimpleError};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("sync.md");
const USAGE: &str = help_usage!("sync.md");

pub mod options {
    pub const FILE_SYSTEM: &str = "file-system";
    pub const DATA: &str = "data";
}

const ARG_FILES: &str = "files";

#[cfg(unix)]
mod platform {
    use nix::{fcntl::OFlag, sys::stat::Mode};
    use std::{
        os::fd::RawFd,
        path::{Path, PathBuf},
    };
    use uucore::{display::Quotable, error::FromIo, error::UResult, show};

    pub fn sync() {
        unsafe { libc::sync() };
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn syncfs(files: &[PathBuf]) {
        for path in files {
            match open(path) {
                Ok(fd) => {
                    let _ = unsafe { libc::syncfs(fd) };
                    let _ = unsafe { libc::close(fd) };
                }
                Err(e) => show!(e),
            }
        }
    }

    pub fn fdatasync(files: &[PathBuf]) {
        for path in files {
            match open(path) {
                Ok(fd) => {
                    let _ = unsafe { libc::fdatasync(fd) };
                    let _ = unsafe { libc::close(fd) };
                }
                Err(e) => show!(e),
            }
        }
    }

    pub fn fsync(files: &[PathBuf]) {
        for path in files {
            match open(path) {
                Ok(fd) => {
                    let _ = unsafe { libc::fsync(fd) };
                    let _ = unsafe { libc::close(fd) };
                }
                Err(e) => show!(e),
            }
        }
    }

    fn open(path: &Path) -> UResult<RawFd> {
        // Use the Nix open to be able to set the NONBLOCK flags for fifo files
        nix::fcntl::open(path, OFlag::O_NONBLOCK, Mode::empty())
            .map_err_context(|| format!("error opening {}", path.quote()))
    }
}

#[cfg(windows)]
mod platform {
    use std::fs::OpenOptions;
    use std::os::windows::prelude::*;
    use std::path::PathBuf;
    use uucore::crash;
    use uucore::wide::{FromWide, ToWide};
    use windows_sys::Win32::Foundation::{
        GetLastError, ERROR_NO_MORE_FILES, HANDLE, INVALID_HANDLE_VALUE, MAX_PATH,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, FlushFileBuffers, GetDriveTypeW,
    };
    use windows_sys::Win32::System::WindowsProgramming::DRIVE_FIXED;

    fn flush_volume(name: &str) {
        let name_wide = name.to_wide_null();
        if unsafe { GetDriveTypeW(name_wide.as_ptr()) } == DRIVE_FIXED {
            let sliced_name = &name[..name.len() - 1]; // eliminate trailing backslash
            match OpenOptions::new().write(true).open(sliced_name) {
                Ok(file) => {
                    if unsafe { FlushFileBuffers(file.as_raw_handle() as HANDLE) } == 0 {
                        crash!(
                            unsafe { GetLastError() } as i32,
                            "failed to flush file buffer"
                        );
                    }
                }
                Err(e) => crash!(
                    e.raw_os_error().unwrap_or(1),
                    "failed to create volume handle"
                ),
            }
        }
    }

    fn find_first_volume() -> (String, HANDLE) {
        let mut name: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
        let handle = unsafe { FindFirstVolumeW(name.as_mut_ptr(), name.len() as u32) };
        if handle == INVALID_HANDLE_VALUE {
            crash!(
                unsafe { GetLastError() } as i32,
                "failed to find first volume"
            );
        }
        (String::from_wide_null(&name), handle)
    }

    fn find_all_volumes() -> Vec<String> {
        let (first_volume, next_volume_handle) = find_first_volume();
        let mut volumes = vec![first_volume];
        loop {
            let mut name: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
            if unsafe { FindNextVolumeW(next_volume_handle, name.as_mut_ptr(), name.len() as u32) }
                == 0
            {
                match unsafe { GetLastError() } {
                    ERROR_NO_MORE_FILES => {
                        unsafe { FindVolumeClose(next_volume_handle) };
                        return volumes;
                    }
                    err => crash!(err as i32, "failed to find next volume"),
                }
            } else {
                volumes.push(String::from_wide_null(&name));
            }
        }
    }

    pub fn sync() {
        let volumes = find_all_volumes();
        for vol in &volumes {
            flush_volume(vol);
        }
    }

    pub fn syncfs(files: &[PathBuf]) {
        for path in files {
            flush_volume(
                path.components()
                    .next()
                    .unwrap()
                    .as_os_str()
                    .to_str()
                    .unwrap(),
            );
        }
    }

    pub fn fsync(_files: &[PathBuf]) {
        todo!()
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let files: Vec<PathBuf> = matches
        .get_many::<PathBuf>(ARG_FILES)
        .map(|v| v.map(ToOwned::to_owned).collect())
        .unwrap_or_default();

    let file_system = matches.get_flag(options::FILE_SYSTEM);
    let data = matches.get_flag(options::DATA);

    #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "windows")))]
    if file_system {
        return Err(USimpleError::new(
            1,
            "--file-system is not supported on this platform",
        ));
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    if data {
        return Err(USimpleError::new(
            1,
            "--data is not supported on this platform",
        ));
    }

    if data && files.is_empty() {
        return Err(USimpleError::new(1, "--data needs at least one argument"));
    }

    #[cfg(any(target_os = "linux", target_os = "android", target_os = "windows"))]
    if file_system {
        platform::syncfs(&files);
        return Ok(());
    }

    #[cfg(unix)]
    if data {
        platform::fdatasync(&files);
        return Ok(());
    }

    if files.is_empty() {
        platform::sync();
    } else {
        platform::fsync(&files);
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE_SYSTEM)
                .short('f')
                .long(options::FILE_SYSTEM)
                .conflicts_with(options::DATA)
                .help("sync the file systems that contain the files (Linux, Android and Windows only)")
                .action(ArgAction::SetTrue)
                .hide(!cfg!(any(
                    target_os = "linux",
                    target_os = "android",
                    target_os = "windows"
                ))),
        )
        .arg(
            Arg::new(options::DATA)
                .short('d')
                .long(options::DATA)
                .conflicts_with(options::FILE_SYSTEM)
                .help("sync only file data, no unneeded metadata (Unix only)")
                .action(ArgAction::SetTrue)
                .hide(!cfg!(unix)),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath)
                .value_parser(clap::value_parser!(PathBuf)),
        )
}
