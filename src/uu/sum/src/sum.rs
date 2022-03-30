// * This file is part of the uutils coreutils package.
// *
// * (c) T. Jameson Little <t.jameson.little@gmail.com>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

// spell-checker:ignore (ToDO) sysv

#[macro_use]
extern crate uucore;

use clap::{crate_version, Arg, Command};
use std::fs::File;
use std::io::{stdin, Read};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::{format_usage, InvalidEncodingHandling};

static NAME: &str = "sum";
static USAGE: &str = "{} [OPTION]... [FILE]...";
static SUMMARY: &str = "Checksum and count the blocks in a file.\n\
                        With no FILE, or when  FILE is -, read standard input.";

fn bsd_sum(mut reader: Box<dyn Read>) -> (usize, u16) {
    let mut buf = [0; 1024];
    let mut blocks_read = 0;
    let mut checksum: u16 = 0;
    loop {
        match reader.read(&mut buf) {
            Ok(n) if n != 0 => {
                blocks_read += 1;
                for &byte in buf[..n].iter() {
                    checksum = (checksum >> 1) + ((checksum & 1) << 15);
                    checksum = checksum.wrapping_add(u16::from(byte));
                }
            }
            _ => break,
        }
    }

    (blocks_read, checksum)
}

fn sysv_sum(mut reader: Box<dyn Read>) -> (usize, u16) {
    let mut buf = [0; 512];
    let mut blocks_read = 0;
    let mut ret = 0u32;

    loop {
        match reader.read(&mut buf) {
            Ok(n) if n != 0 => {
                blocks_read += 1;
                for &byte in buf[..n].iter() {
                    ret = ret.wrapping_add(u32::from(byte));
                }
            }
            _ => break,
        }
    }

    ret = (ret & 0xffff) + (ret >> 16);
    ret = (ret & 0xffff) + (ret >> 16);

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
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let matches = uu_app().get_matches_from(args);

    let files: Vec<String> = match matches.values_of(options::FILE) {
        Some(v) => v.clone().map(|v| v.to_owned()).collect(),
        None => vec!["-".to_owned()],
    };

    let sysv = matches.is_present(options::SYSTEM_V_COMPATIBLE);

    let print_names = if sysv {
        files.len() > 1 || files[0] != "-"
    } else {
        files.len() > 1
    };

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
            println!("{} {} {}", sum, blocks, file);
        } else {
            println!("{} {}", sum, blocks);
        }
    }
    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .name(NAME)
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(SUMMARY)
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE)
                .multiple_occurrences(true)
                .hide(true),
        )
        .arg(
            Arg::new(options::BSD_COMPATIBLE)
                .short('r')
                .help("use the BSD sum algorithm, use 1K blocks (default)"),
        )
        .arg(
            Arg::new(options::SYSTEM_V_COMPATIBLE)
                .short('s')
                .long(options::SYSTEM_V_COMPATIBLE)
                .help("use System V sum algorithm, use 512 bytes blocks"),
        )
}
