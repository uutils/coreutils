// * This file is part of the uutils coreutils package.
// *
// * (c) T. Jameson Little <t.jameson.little@gmail.com>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

// spell-checker:ignore (ToDO) sysv

#[macro_use]
extern crate uucore;

use std::fs::File;
use std::io::{stdin, Read, Result};
use std::path::Path;
use uucore::InvalidEncodingHandling;

use crate::app::{get_app, options};

pub mod app;

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

fn open(name: &str) -> Result<Box<dyn Read>> {
    match name {
        "-" => Ok(Box::new(stdin()) as Box<dyn Read>),
        _ => {
            let path = &Path::new(name);
            if path.is_dir() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Is a directory",
                ));
            };
            if path.metadata().is_err() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "No such file or directory",
                ));
            };
            let f = File::open(path)?;
            Ok(Box::new(f) as Box<dyn Read>)
        }
    }
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let matches = get_app(executable!()).get_matches_from(args);

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

    let mut exit_code = 0;
    for file in &files {
        let reader = match open(file) {
            Ok(f) => f,
            Err(error) => {
                show_error!("'{}' {}", file, error);
                exit_code = 2;
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

    exit_code
}
