// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) sysv

use clap::{crate_version, Arg, ArgAction, Command};
use std::fs::File;
use std::io::{stdin, Read};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::{format_usage, help_about, help_usage, show};

const USAGE: &str = help_usage!("sum.md");
const ABOUT: &str = help_about!("sum.md");

// This can be replaced with usize::div_ceil once it is stabilized.
// This implementation approach is optimized for when `b` is a constant,
// particularly a power of two.
const fn div_ceil(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}

fn bsd_sum(mut reader: Box<dyn Read>) -> (usize, u16) {
    let mut buf = [0; 4096];
    let mut bytes_read = 0;
    let mut checksum: u16 = 0;
    loop {
        match reader.read(&mut buf) {
            Ok(n) if n != 0 => {
                bytes_read += n;
                for &byte in &buf[..n] {
                    checksum = checksum.rotate_right(1);
                    checksum = checksum.wrapping_add(u16::from(byte));
                }
            }
            _ => break,
        }
    }

    // Report blocks read in terms of 1024-byte blocks.
    let blocks_read = div_ceil(bytes_read, 1024);
    (blocks_read, checksum)
}

fn sysv_sum(mut reader: Box<dyn Read>) -> (usize, u16) {
    let mut buf = [0; 4096];
    let mut bytes_read = 0;
    let mut ret = 0u32;

    loop {
        match reader.read(&mut buf) {
            Ok(n) if n != 0 => {
                bytes_read += n;
                for &byte in &buf[..n] {
                    ret = ret.wrapping_add(u32::from(byte));
                }
            }
            _ => break,
        }
    }

    ret = (ret & 0xffff) + (ret >> 16);
    ret = (ret & 0xffff) + (ret >> 16);

    // Report blocks read in terms of 512-byte blocks.
    let blocks_read = div_ceil(bytes_read, 512);
    (blocks_read, ret as u16)
}

fn open(name: &str) -> UResult<Box<dyn Read>> {
    match name {
        "-" => Ok(Box::new(stdin()) as Box<dyn Read>),
        _ => {
            let path = &Path::new(name);
            if path.is_dir() {
                return Err(USimpleError::new(
                    2,
                    format!("{}: Is a directory", name.maybe_quote()),
                ));
            };
            // Silent the warning as we want to the error message
            if path.metadata().is_err() {
                return Err(USimpleError::new(
                    2,
                    format!("{}: No such file or directory", name.maybe_quote()),
                ));
            };
            let f = File::open(path).map_err_context(String::new)?;
            Ok(Box::new(f) as Box<dyn Read>)
        }
    }
}

mod options {
    pub static FILE: &str = "file";
    pub static BSD_COMPATIBLE: &str = "r";
    pub static SYSTEM_V_COMPATIBLE: &str = "sysv";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let files: Vec<String> = match matches.get_many::<String>(options::FILE) {
        Some(v) => v.cloned().collect(),
        None => vec!["-".to_owned()],
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
        };

        if print_names {
            println!("{sum:0width$} {blocks:width$} {file}");
        } else {
            println!("{sum:0width$} {blocks:width$}");
        }
    }
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(ABOUT)
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE)
                .action(ArgAction::Append)
                .hide(true)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::BSD_COMPATIBLE)
                .short('r')
                .help("use the BSD sum algorithm, use 1K blocks (default)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SYSTEM_V_COMPATIBLE)
                .short('s')
                .long(options::SYSTEM_V_COMPATIBLE)
                .help("use System V sum algorithm, use 512 bytes blocks")
                .action(ArgAction::SetTrue),
        )
}
