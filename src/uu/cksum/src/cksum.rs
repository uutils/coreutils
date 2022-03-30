// This file is part of the uutils coreutils package.
//
// (c) Michael Gehring <mg@ebfe.org>
//
//  For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fname
use clap::{crate_version, Arg, Command};
use std::fs::File;
use std::io::{self, stdin, BufReader, Read};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult};
use uucore::InvalidEncodingHandling;
use uucore::{format_usage, show};

// NOTE: CRC_TABLE_LEN *must* be <= 256 as we cast 0..CRC_TABLE_LEN to u8
const CRC_TABLE_LEN: usize = 256;
const CRC_TABLE: [u32; CRC_TABLE_LEN] = generate_crc_table();

const NAME: &str = "cksum";
const USAGE: &str = "{} [OPTIONS] [FILE]...";
const SUMMARY: &str = "Print CRC and size for each file";

const fn generate_crc_table() -> [u32; CRC_TABLE_LEN] {
    let mut table = [0; CRC_TABLE_LEN];

    let mut i = 0;
    while i < CRC_TABLE_LEN {
        table[i] = crc_entry(i as u8) as u32;

        i += 1;
    }

    table
}

const fn crc_entry(input: u8) -> u32 {
    let mut crc = (input as u32) << 24;

    let mut i = 0;
    while i < 8 {
        let if_condition = crc & 0x8000_0000;
        let if_body = (crc << 1) ^ 0x04c1_1db7;
        let else_body = crc << 1;

        // NOTE: i feel like this is easier to understand than emulating an if statement in bitwise
        //       ops
        let condition_table = [else_body, if_body];

        crc = condition_table[(if_condition != 0) as usize];
        i += 1;
    }

    crc
}

#[inline]
fn crc_update(crc: u32, input: u8) -> u32 {
    (crc << 8) ^ CRC_TABLE[((crc >> 24) as usize ^ input as usize) & 0xFF]
}

#[inline]
fn crc_final(mut crc: u32, mut length: usize) -> u32 {
    while length != 0 {
        crc = crc_update(crc, length as u8);
        length >>= 8;
    }

    !crc
}

fn init_byte_array() -> Vec<u8> {
    vec![0; 1024 * 1024]
}

#[inline]
fn cksum(fname: &str) -> io::Result<(u32, usize)> {
    let mut crc = 0u32;
    let mut size = 0usize;

    let mut rd: Box<dyn Read> = match fname {
        "-" => Box::new(stdin()),
        _ => {
            let p = Path::new(fname);

            // Directories should not give an error, but should be interpreted
            // as empty files to match GNU semantics.
            if p.is_dir() {
                Box::new(BufReader::new(io::empty())) as Box<dyn Read>
            } else {
                Box::new(BufReader::new(File::open(p)?)) as Box<dyn Read>
            }
        }
    };

    let mut bytes = init_byte_array();
    loop {
        let num_bytes = rd.read(&mut bytes)?;
        if num_bytes == 0 {
            return Ok((crc_final(crc, size), size));
        }
        for &b in bytes[..num_bytes].iter() {
            crc = crc_update(crc, b);
        }
        size += num_bytes;
    }
}

mod options {
    pub static FILE: &str = "file";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let matches = uu_app().get_matches_from(args);

    let files: Vec<String> = match matches.values_of(options::FILE) {
        Some(v) => v.clone().map(|v| v.to_owned()).collect(),
        None => vec![],
    };

    if files.is_empty() {
        let (crc, size) = cksum("-")?;
        println!("{} {}", crc, size);
        return Ok(());
    }

    for fname in &files {
        match cksum(fname.as_ref()).map_err_context(|| format!("{}", fname.maybe_quote())) {
            Ok((crc, size)) => println!("{} {} {}", crc, size, fname),
            Err(err) => show!(err),
        };
    }
    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .name(NAME)
        .version(crate_version!())
        .about(SUMMARY)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .multiple_occurrences(true),
        )
}
