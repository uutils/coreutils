// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ttyname filedesc

#![cfg(not(target_os = "fuchsia"))]

use clap::{Arg, ArgAction, Command};
use std::io::{IsTerminal, Write};
use uucore::error::{UResult, set_exit_code, strip_errno};
use uucore::{format_usage, show_error, translate};

mod options {
    pub const SILENT: &str = "silent";
}

#[uucore::main(no_signals)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result_with_exit_code(uu_app(), args, 2)?;

    // Disable SIGPIPE so we can handle broken pipe errors gracefully
    // and exit with code 3 instead of being killed by the signal.
    #[cfg(unix)]
    let _ = uucore::signals::disable_pipe_errors();

    let silent = matches.get_flag(options::SILENT);

    // If silent, we don't need the name, only whether or not stdin is a tty.
    if silent {
        return if std::io::stdin().is_terminal() {
            Ok(())
        } else {
            Err(1.into())
        };
    }

    let mut stdout = std::io::stdout();
    #[cfg(unix)]
    let name = rustix::termios::ttyname(std::io::stdin(), Vec::with_capacity(8));
    #[cfg(unix)]
    let write_result = if let Ok(name) = name {
        let mut buf = name.as_bytes().to_vec();
        buf.push(b'\n');
        stdout.write_all(&buf)
    } else {
        set_exit_code(1);
        writeln!(stdout, "{}", translate!("tty-not-a-tty"))
    };
    #[cfg(target_os = "wasi")]
    let write_result = if std::io::stdin().is_terminal() {
        // maximize compatibility
        writeln!(stdout, r"/dev/tty")
    } else {
        set_exit_code(1);
        writeln!(stdout, "{}", translate!("tty-not-a-tty"))
    };
    #[cfg(windows)]
    let write_result = {
        use std::os::windows::io::AsHandle;
        let stdin = std::io::stdin();
        let stdin_handle = stdin.as_handle();
        if stdin_handle.is_terminal() {
            writeln!(
                stdout,
                "{}",
                file_name(stdin_handle).as_deref().unwrap_or(r"\\.\CON")
            )
        } else {
            set_exit_code(1);
            writeln!(stdout, "{}", translate!("tty-not-a-tty"))
        }
    };

    if let Err(e) = write_result {
        show_error!("{}", strip_errno(&e));
        set_exit_code(3);
    }

    Ok(())
}

#[cfg(windows)]
fn file_name(handle: std::os::windows::io::BorrowedHandle) -> Option<String> {
    // This code is adapted from rust's standard library
    // https://github.com/rust-lang/rust/blob/0424cc16731e6141a18077f8ccde77ba148d9649/library/std/src/sys/io/is_terminal/windows.rs#L25
    use std::mem::MaybeUninit;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::MAX_PATH;
    use windows_sys::Win32::Storage::FileSystem::{FileNameInfo, GetFileInformationByHandleEx};
    // Manually define FILE_NAME_INFO so we can more easily construct a stack buffer.
    #[repr(C)]
    #[allow(non_snake_case)]
    struct FILE_NAME_INFO {
        FileNameLength: u32,
        FileName: [MaybeUninit<u16>; MAX_PATH as usize],
    }
    let mut name = FILE_NAME_INFO {
        FileNameLength: 0,
        FileName: [MaybeUninit::uninit(); MAX_PATH as usize],
    };
    unsafe {
        let result = GetFileInformationByHandleEx(
            handle.as_raw_handle(),
            FileNameInfo,
            (&raw mut name).cast(),
            size_of::<FILE_NAME_INFO>() as u32,
        );
        if result == 0 {
            None
        } else {
            let name = name.FileName.get(..name.FileNameLength as usize / 2)?;
            // SAFETY: all elements up to FileNameLength have been initialized
            let name: &[u16] = &*(std::ptr::from_ref::<[MaybeUninit<u16>]>(name) as *const [u16]);
            // This should never fail for a valid msys terminal because they use ASCII names.
            String::from_utf16(name).ok()
        }
    }
}

pub fn uu_app() -> Command {
    let cmd = Command::new("tty")
        .version(uucore::crate_version!())
        .about(translate!("tty-about"))
        .override_usage(format_usage(&translate!("tty-usage")))
        .infer_long_args(true);
    uucore::clap_localization::configure_localized_command(cmd).arg(
        Arg::new(options::SILENT)
            .long(options::SILENT)
            .visible_alias("quiet")
            .short('s')
            .help(translate!("tty-help-silent"))
            .action(ArgAction::SetTrue),
    )
}
