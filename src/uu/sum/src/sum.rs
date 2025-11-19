// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) sysv

use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::fs::File;
use std::io::{ErrorKind, Read, Write, stdin, stdout};
use std::path::Path;
use uucore::display::{OsWrite, Quotable};
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::translate;

use uucore::{format_usage, show};

fn bsd_sum(mut reader: impl Read) -> std::io::Result<(usize, u16)> {
    let mut buf = [0; 4096];
    let mut bytes_read = 0;
    let mut checksum: u16 = 0;
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                bytes_read += n;
                checksum = buf[..n].iter().fold(checksum, |acc, &byte| {
                    let rotated = acc.rotate_right(1);
                    rotated.wrapping_add(u16::from(byte))
                });
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => (),
            Err(e) => return Err(e),
        }
    }

    // Report blocks read in terms of 1024-byte blocks.
    let blocks_read = bytes_read.div_ceil(1024);
    Ok((blocks_read, checksum))
}

fn sysv_sum(mut reader: impl Read) -> std::io::Result<(usize, u16)> {
    let mut buf = [0; 4096];
    let mut bytes_read = 0;
    let mut ret = 0u32;

    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                bytes_read += n;
                ret = buf[..n]
                    .iter()
                    .fold(ret, |acc, &byte| acc.wrapping_add(u32::from(byte)));
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => (),
            Err(e) => return Err(e),
        }
    }

    ret = (ret & 0xffff) + (ret >> 16);
    ret = (ret & 0xffff) + (ret >> 16);

    // Report blocks read in terms of 512-byte blocks.
    let blocks_read = bytes_read.div_ceil(512);
    Ok((blocks_read, ret as u16))
}

fn open(name: &OsString) -> UResult<Box<dyn Read>> {
    if name == "-" {
        Ok(Box::new(stdin()) as Box<dyn Read>)
    } else {
        let path = Path::new(name);
        if path.is_dir() {
            return Err(USimpleError::new(
                2,
                translate!("sum-error-is-directory", "name" => name.maybe_quote()),
            ));
        }
        // Silent the warning as we want to the error message
        if path.metadata().is_err() {
            return Err(USimpleError::new(
                2,
                translate!("sum-error-no-such-file-or-directory", "name" => name.maybe_quote()),
            ));
        }
        let f = File::open(path).map_err_context(String::new)?;
        Ok(Box::new(f) as Box<dyn Read>)
    }
}

mod options {
    pub static FILE: &str = "file";
    pub static BSD_COMPATIBLE: &str = "r";
    pub static SYSTEM_V_COMPATIBLE: &str = "sysv";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let files: Vec<OsString> = match matches.get_many::<OsString>(options::FILE) {
        Some(v) => v.cloned().collect(),
        None => vec![OsString::from("-")],
    };

    let sysv = matches.get_flag(options::SYSTEM_V_COMPATIBLE);

    let print_names = files.len() > 1 || files[0] != "-";
    let width = if sysv { 1 } else { 5 };

    for file in &files {
        let reader = match open(file) {
            Ok(f) => f,
            Err(error) => {
                show!(error);
                continue;
            }
        };
        let (blocks, sum) = if sysv {
            sysv_sum(reader)
        } else {
            bsd_sum(reader)
        }?;

        let mut stdout = stdout().lock();
        if print_names {
            write!(stdout, "{sum:0width$} {blocks:width$} ")?;
            stdout.write_all_os(file)?;
            stdout.write_all(b"\n")?;
        } else {
            writeln!(stdout, "{sum:0width$} {blocks:width$}")?;
        }
    }
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(format_usage(&translate!("sum-usage")))
        .about(translate!("sum-about"))
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE)
                .action(ArgAction::Append)
                .hide(true)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::BSD_COMPATIBLE)
                .short('r')
                .help(translate!("sum-help-bsd-compatible"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SYSTEM_V_COMPATIBLE)
                .short('s')
                .long(options::SYSTEM_V_COMPATIBLE)
                .help(translate!("sum-help-sysv-compatible"))
                .action(ArgAction::SetTrue),
        )
}
