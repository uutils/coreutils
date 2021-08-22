//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Alex Lyon <arcterus@mail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) sbytes slen dlen

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg};
use std::io::{stdin, stdout, BufReader, Read, Write};
use std::{fs::File, path::Path};
use uucore::InvalidEncodingHandling;

static NAME: &str = "tac";
static USAGE: &str = "[OPTION]... [FILE]...";
static SUMMARY: &str = "Write each file to standard output, last line first.";

mod options {
    pub static BEFORE: &str = "before";
    pub static REGEX: &str = "regex";
    pub static SEPARATOR: &str = "separator";
    pub static FILE: &str = "file";
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let matches = uu_app().get_matches_from(args);

    let before = matches.is_present(options::BEFORE);
    let regex = matches.is_present(options::REGEX);
    let raw_separator = matches.value_of(options::SEPARATOR).unwrap_or("\n");
    let separator = if raw_separator.is_empty() {
        "\0"
    } else {
        raw_separator
    };

    let files: Vec<String> = match matches.values_of(options::FILE) {
        Some(v) => v.map(|v| v.to_owned()).collect(),
        None => vec!["-".to_owned()],
    };

    tac(files, before, regex, separator)
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .name(NAME)
        .version(crate_version!())
        .usage(USAGE)
        .about(SUMMARY)
        .arg(
            Arg::with_name(options::BEFORE)
                .short("b")
                .long(options::BEFORE)
                .help("attach the separator before instead of after")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::REGEX)
                .short("r")
                .long(options::REGEX)
                .help("interpret the sequence as a regular expression (NOT IMPLEMENTED)")
                .takes_value(false),
        )
        .arg(
            Arg::with_name(options::SEPARATOR)
                .short("s")
                .long(options::SEPARATOR)
                .help("use STRING as the separator instead of newline")
                .takes_value(true),
        )
        .arg(Arg::with_name(options::FILE).hidden(true).multiple(true))
}

fn buffer_tac(data: &[u8], before: bool, separator: &str) -> std::io::Result<()> {
    let mut out = stdout();

    // Convert the line separator to a byte sequence.
    let sbytes = separator.as_bytes();
    let slen = sbytes.len();

    // If there are more characters in the separator than in the data,
    // we can't possibly split the data on the separator. Write the
    // entire buffer to stdout.
    let dlen = data.len();
    if dlen < slen {
        return out.write_all(data);
    }

    // Iterate over each byte in the buffer in reverse. When we find a
    // line separator, write the line to stdout.
    //
    // The `before` flag controls whether the line separator appears at
    // the end of the line (as in "abc\ndef\n") or at the beginning of
    // the line (as in "/abc/def").
    let mut following_line_start = data.len();
    for i in (0..dlen - slen + 1).rev() {
        if &data[i..i + slen] == sbytes {
            if before {
                out.write_all(&data[i..following_line_start])?;
                following_line_start = i;
            } else {
                out.write_all(&data[i + slen..following_line_start])?;
                following_line_start = i + slen;
            }
        }
    }

    // After the loop terminates, write whatever bytes are remaining at
    // the beginning of the buffer.
    out.write_all(&data[0..following_line_start])?;
    Ok(())
}

fn tac(filenames: Vec<String>, before: bool, _: bool, separator: &str) -> i32 {
    let mut exit_code = 0;

    for filename in &filenames {
        let mut file = BufReader::new(if filename == "-" {
            Box::new(stdin()) as Box<dyn Read>
        } else {
            let path = Path::new(filename);
            if path.is_dir() || path.metadata().is_err() {
                if path.is_dir() {
                    show_error!("{}: read error: Invalid argument", filename);
                } else {
                    show_error!(
                        "failed to open '{}' for reading: No such file or directory",
                        filename
                    );
                }
                exit_code = 1;
                continue;
            }
            match File::open(path) {
                Ok(f) => Box::new(f) as Box<dyn Read>,
                Err(e) => {
                    show_error!("failed to open '{}' for reading: {}", filename, e);
                    exit_code = 1;
                    continue;
                }
            }
        });

        let mut data = Vec::new();
        if let Err(e) = file.read_to_end(&mut data) {
            show_error!("failed to read '{}': {}", filename, e);
            exit_code = 1;
            continue;
        };

        buffer_tac(&data, before, separator)
            .unwrap_or_else(|e| crash!(1, "failed to write to stdout: {}", e));
    }
    exit_code
}
