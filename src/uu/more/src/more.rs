//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Martin Kysel <code@martinkysel.com>
//  *
//  * For the full copyright and license information, please view the LICENSE file
//  * that was distributed with this source code.

// spell-checker:ignore (methods) isnt

#[macro_use]
extern crate uucore;

use std::{
    convert::TryInto,
    fs::File,
    io::{stdin, stdout, BufReader, Read, Stdout, Write},
    path::Path,
    time::Duration,
};

#[cfg(all(unix, not(target_os = "fuchsia")))]
extern crate nix;

use clap::{crate_version, App, Arg};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::Attribute,
    terminal,
};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub mod options {
    pub const SILENT: &str = "silent";
    pub const LOGICAL: &str = "logical";
    pub const NO_PAUSE: &str = "no-pause";
    pub const PRINT_OVER: &str = "print-over";
    pub const CLEAN_PRINT: &str = "clean-print";
    pub const SQUEEZE: &str = "squeeze";
    pub const PLAIN: &str = "plain";
    pub const LINES: &str = "lines";
    pub const NUMBER: &str = "number";
    pub const PATTERN: &str = "pattern";
    pub const FROM_LINE: &str = "from-line";
    pub const FILES: &str = "files";
}

const MULTI_FILE_TOP_PROMPT: &str = "::::::::::::::\n{}\n::::::::::::::\n";

pub fn uumain(args: impl uucore::Args) -> i32 {
    let matches = App::new(executable!())
        .about("A file perusal filter for CRT viewing.")
        .version(crate_version!())
        // The commented arguments below are unimplemented:
        /*
        .arg(
            Arg::with_name(options::SILENT)
                .short("d")
                .long(options::SILENT)
                .help("Display help instead of ringing bell"),
        )
        .arg(
            Arg::with_name(options::LOGICAL)
                .short("f")
                .long(options::LOGICAL)
                .help("Count logical rather than screen lines"),
        )
        .arg(
            Arg::with_name(options::NO_PAUSE)
                .short("l")
                .long(options::NO_PAUSE)
                .help("Suppress pause after form feed"),
        )
        .arg(
            Arg::with_name(options::PRINT_OVER)
                .short("c")
                .long(options::PRINT_OVER)
                .help("Do not scroll, display text and clean line ends"),
        )
        .arg(
            Arg::with_name(options::CLEAN_PRINT)
                .short("p")
                .long(options::CLEAN_PRINT)
                .help("Do not scroll, clean screen and display text"),
        )
        .arg(
            Arg::with_name(options::SQUEEZE)
                .short("s")
                .long(options::SQUEEZE)
                .help("Squeeze multiple blank lines into one"),
        )
        .arg(
            Arg::with_name(options::PLAIN)
                .short("u")
                .long(options::PLAIN)
                .help("Suppress underlining and bold"),
        )
        .arg(
            Arg::with_name(options::LINES)
                .short("n")
                .long(options::LINES)
                .value_name("number")
                .takes_value(true)
                .help("The number of lines per screen full"),
        )
        .arg(
            Arg::with_name(options::NUMBER)
                .allow_hyphen_values(true)
                .long(options::NUMBER)
                .required(false)
                .takes_value(true)
                .help("Same as --lines"),
        )
        .arg(
            Arg::with_name(options::FROM_LINE)
                .short("F")
                .allow_hyphen_values(true)
                .required(false)
                .takes_value(true)
                .value_name("number")
                .help("Display file beginning from line number"),
        )
        .arg(
            Arg::with_name(options::PATTERN)
                .short("P")
                .allow_hyphen_values(true)
                .required(false)
                .takes_value(true)
                .help("Display file beginning from pattern match"),
        )
        */
        .arg(
            Arg::with_name(options::FILES)
                .required(false)
                .multiple(true)
                .help("Path to the files to be read"),
        )
        .get_matches_from(args);

    let mut buff = String::new();
    if let Some(files) = matches.values_of(options::FILES) {
        let mut stdout = setup_term();
        let length = files.len();

        let mut files_iter = files.peekable();
        while let (Some(file), next_file) = (files_iter.next(), files_iter.peek()) {
            let file = Path::new(file);
            if file.is_dir() {
                terminal::disable_raw_mode().unwrap();
                show_usage_error!("'{}' is a directory.", file.display());
                return 1;
            }
            if !file.exists() {
                terminal::disable_raw_mode().unwrap();
                show_error!("cannot open {}: No such file or directory", file.display());
                return 1;
            }
            if length > 1 {
                buff.push_str(&MULTI_FILE_TOP_PROMPT.replace("{}", file.to_str().unwrap()));
            }
            let mut reader = BufReader::new(File::open(file).unwrap());
            reader.read_to_string(&mut buff).unwrap();
            more(&buff, &mut stdout, next_file.copied());
            buff.clear();
        }
        reset_term(&mut stdout);
    } else if atty::isnt(atty::Stream::Stdin) {
        stdin().read_to_string(&mut buff).unwrap();
        let mut stdout = setup_term();
        more(&buff, &mut stdout, None);
        reset_term(&mut stdout);
    } else {
        show_usage_error!("bad usage");
    }
    0
}

#[cfg(not(target_os = "fuchsia"))]
fn setup_term() -> std::io::Stdout {
    let stdout = stdout();
    terminal::enable_raw_mode().unwrap();
    stdout
}

#[cfg(target_os = "fuchsia")]
#[inline(always)]
fn setup_term() -> usize {
    0
}

#[cfg(not(target_os = "fuchsia"))]
fn reset_term(stdout: &mut std::io::Stdout) {
    terminal::disable_raw_mode().unwrap();
    // Clear the prompt
    queue!(stdout, terminal::Clear(terminal::ClearType::CurrentLine)).unwrap();
    // Move cursor to the beginning without printing new line
    print!("\r");
    stdout.flush().unwrap();
}

#[cfg(target_os = "fuchsia")]
#[inline(always)]
fn reset_term(_: &mut usize) {}

fn more(buff: &str, mut stdout: &mut Stdout, next_file: Option<&str>) {
    let (cols, rows) = terminal::size().unwrap();
    let lines = break_buff(buff, usize::from(cols));
    let line_count: u16 = lines.len().try_into().unwrap();

    let mut upper_mark = 0;
    let mut lines_left = line_count.saturating_sub(upper_mark + rows);

    draw(
        upper_mark,
        rows,
        &mut stdout,
        lines.clone(),
        line_count,
        next_file,
    );

    let is_last = next_file.is_none();

    // Specifies whether we have reached the end of the file and should
    // return on the next key press. However, we immediately return when
    // this is the last file.
    let mut to_be_done = false;
    if lines_left == 0 && is_last {
        if is_last {
            return;
        } else {
            to_be_done = true;
        }
    }

    loop {
        if event::poll(Duration::from_millis(10)).unwrap() {
            match event::read().unwrap() {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    modifiers: KeyModifiers::NONE,
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                }) => {
                    reset_term(&mut stdout);
                    std::process::exit(0);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    modifiers: KeyModifiers::NONE,
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char(' '),
                    modifiers: KeyModifiers::NONE,
                }) => {
                    upper_mark = upper_mark.saturating_add(rows.saturating_sub(1));
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::NONE,
                }) => {
                    upper_mark = upper_mark.saturating_sub(rows.saturating_sub(1));
                }
                _ => continue,
            }
            lines_left = line_count.saturating_sub(upper_mark + rows);
            draw(
                upper_mark,
                rows,
                &mut stdout,
                lines.clone(),
                line_count,
                next_file,
            );

            if lines_left == 0 {
                if to_be_done || is_last {
                    return;
                }
                to_be_done = true;
            }
        }
    }
}

fn draw(
    upper_mark: u16,
    rows: u16,
    mut stdout: &mut std::io::Stdout,
    lines: Vec<String>,
    lc: u16,
    next_file: Option<&str>,
) {
    execute!(stdout, terminal::Clear(terminal::ClearType::CurrentLine)).unwrap();
    let lower_mark = calc_lower_mark(upper_mark, rows, lc);
    // Reduce the row by 1 for the prompt
    let displayed_lines = lines
        .iter()
        .skip(upper_mark.into())
        .take(usize::from(rows.saturating_sub(1)));

    for line in displayed_lines {
        stdout
            .write_all(format!("\r{}\n", line).as_bytes())
            .unwrap();
    }
    make_prompt_and_flush(&mut stdout, lower_mark, lc, next_file);
}

// Break the lines on the cols of the terminal
fn break_buff(buff: &str, cols: usize) -> Vec<String> {
    let mut lines = Vec::new();

    for l in buff.lines() {
        lines.append(&mut break_line(l, cols));
    }
    lines
}

fn break_line(line: &str, cols: usize) -> Vec<String> {
    let width = UnicodeWidthStr::width(line);
    let mut lines = Vec::new();
    if width < cols {
        lines.push(line.to_string());
        return lines;
    }

    let gr_idx = UnicodeSegmentation::grapheme_indices(line, true);
    let mut last_index = 0;
    let mut total_width = 0;
    for (index, grapheme) in gr_idx {
        let width = UnicodeWidthStr::width(grapheme);
        total_width += width;

        if total_width > cols {
            lines.push(line[last_index..index].to_string());
            last_index = index;
            total_width = width;
        }
    }

    if last_index != line.len() {
        lines.push(line[last_index..].to_string());
    }
    lines
}

// Calculate upper_mark based on certain parameters
fn calc_lower_mark(upper_mark: u16, rows: u16, line_count: u16) -> u16 {
    line_count.min(upper_mark + rows - 1)
}

// Make a prompt similar to original more
fn make_prompt_and_flush(stdout: &mut Stdout, lower_mark: u16, lc: u16, next_file: Option<&str>) {
    let status = if lower_mark == lc {
        format!("Next file: {}", next_file.unwrap_or_default())
    } else {
        format!(
            "{}%",
            (lower_mark as f64 / lc as f64 * 100.0).round() as u16
        )
    };
    write!(
        stdout,
        "\r{}--More--({}){}",
        Attribute::Reverse,
        status,
        Attribute::Reset
    )
    .unwrap();
    stdout.flush().unwrap();
}

#[cfg(test)]
mod tests {
    use super::{break_line, calc_lower_mark};
    use unicode_width::UnicodeWidthStr;

    // It is good to test the above functions
    #[test]
    fn test_calc_range() {
        assert_eq!(24, calc_lower_mark(0, 25, 100));
        assert_eq!(74, calc_lower_mark(50, 25, 100));
        assert_eq!(100, calc_lower_mark(85, 25, 100));
        assert_eq!(1, calc_lower_mark(0, 25, 1));
        assert_eq!(24, calc_lower_mark(0, 25, 24));
        assert_eq!(23, calc_lower_mark(0, 25, 23));
    }
    #[test]
    fn test_break_lines_long() {
        let mut test_string = String::with_capacity(100);
        for _ in 0..200 {
            test_string.push('#');
        }

        let lines = break_line(&test_string, 80);
        let widths: Vec<usize> = lines
            .iter()
            .map(|s| UnicodeWidthStr::width(&s[..]))
            .collect();

        assert_eq!((80, 80, 40), (widths[0], widths[1], widths[2]));
    }

    #[test]
    fn test_break_lines_short() {
        let mut test_string = String::with_capacity(100);
        for _ in 0..20 {
            test_string.push('#');
        }

        let lines = break_line(&test_string, 80);

        assert_eq!(20, lines[0].len());
    }

    #[test]
    fn test_break_line_zwj() {
        let mut test_string = String::with_capacity(1100);
        for _ in 0..20 {
            test_string.push_str("👩🏻‍🔬");
        }

        let lines = break_line(&test_string, 80);

        let widths: Vec<usize> = lines
            .iter()
            .map(|s| UnicodeWidthStr::width(&s[..]))
            .collect();

        // Each 👩🏻‍🔬 is 6 character width it break line to the closest number to 80 => 6 * 13 = 78
        assert_eq!((78, 42), (widths[0], widths[1]));
    }
}
