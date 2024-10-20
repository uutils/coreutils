// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore tbody

use std::{
    env,
    error::Error,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
    process::ExitCode,
};

fn main() -> ExitCode {
    if let Err(bo) = write_perfect_hash_map() {
        eprintln!("Error occurred while building and writing perfect hash map: {bo:?}");

        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn write_perfect_hash_map() -> Result<(), Box<dyn Error>> {
    const NUMBER_OF_VALUE_LINES_IN_PRELUDE: usize = 3_usize;

    // `textContent` value of <tbody> node of the table with the heading
    // "Table: Circumflex Control Characters in stty"
    // in https://pubs.opengroup.org/onlinepubs/9799919799/utilities/stty.html
    let text_content = fs::read_to_string("./circumflex-control-characters.txt")?;

    let mut map = phf_codegen::Map::<u8>::new();

    let mut value_encountered_count = 0_usize;

    let mut peekable = text_content.lines().peekable();

    while let Some(current_line) = peekable.peek().copied() {
        peekable.next();

        if current_line.is_empty() || current_line == "\u{00A0}" {
            continue;
        }

        if value_encountered_count < NUMBER_OF_VALUE_LINES_IN_PRELUDE {
            if current_line == "Value" {
                value_encountered_count += 1_usize;
            }

            continue;
        }

        if value_encountered_count != NUMBER_OF_VALUE_LINES_IN_PRELUDE {
            return Err(Box::from("Expectation violated"));
        }

        let keys = current_line;

        loop {
            let Some(str) = peekable.next() else {
                return Err(Box::from("Expectation violated"));
            };

            if str.is_empty() {
                continue;
            }

            let mut encoding_buffer = [0_u8; 4_usize];

            let mut str_chars = str.chars();

            if str_chars.next() != Some('<') || str_chars.next_back() != Some('>') {
                return Err(Box::from("Expectation violated"));
            }

            let str_chars_str = str_chars.as_str();

            // Look up value of alias
            let Some(value_char) = unicode_names2::character(str_chars_str) else {
                return Err(Box::from("Expectation violated"));
            };

            let value_encode_str = value_char.encode_utf8(&mut encoding_buffer);

            let value_only_byte = match value_encode_str.as_bytes() {
                &[ue] => ue,
                _ => {
                    return Err(Box::from("Expectation violated"));
                }
            };

            let value_only_byte_string = format!("{value_only_byte}_u8");

            let value_only_byte_str = value_only_byte_string.as_str();

            for key in keys.split(",") {
                let mut key_chars_skip_while = key.chars().skip_while(|&ch| ch == ' ');

                let Some(key_only_char) = key_chars_skip_while.next() else {
                    return Err(Box::from("Expectation violated"));
                };

                let None = key_chars_skip_while.next() else {
                    return Err(Box::from("Expectation violated"));
                };

                let key_only_char_str = key_only_char.encode_utf8(&mut encoding_buffer);

                match key_only_char_str.as_bytes() {
                    &[key_only_char_only_byte] => {
                        map.entry(key_only_char_only_byte, value_only_byte_str);
                    }
                    _ => {
                        return Err(Box::from("Expectation violated"));
                    }
                }
            }

            break;
        }
    }

    let display_map = map.build();

    let out_dir = env::var("OUT_DIR")?;

    let path_buf = Path::new(out_dir.as_str()).join("circumflex_control_characters_table.rs");

    let mut buf_writer = BufWriter::new(File::create(path_buf.as_path())?);

    writeln!(
        &mut buf_writer,
        "pub static CIRCUMFLEX_CONTROL_CHARACTERS_TABLE_MAP: phf::Map<u8, u8> = {};",
        display_map
    )?;

    Ok(())
}
