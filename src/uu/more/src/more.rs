//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Martin Kysel <code@martinkysel.com>
//  *
//  * For the full copyright and license information, please view the LICENSE file
//  * that was distributed with this source code.

// spell-checker:ignore (methods) isnt

use std::{
    fs::File,
    io::{stdin, stdout, BufReader, Read, Stdout, Write},
    path::Path,
    time::Duration,
};

#[cfg(all(unix, not(target_os = "fuchsia")))]
extern crate nix;

use clap::{crate_version, Arg, Command};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::Attribute,
    terminal,
};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError, UUsageError};

const BELL: &str = "\x07";

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

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let mut buff = String::new();
    let silent = matches.is_present(options::SILENT);
    if let Some(files) = matches.values_of(options::FILES) {
        let mut stdout = setup_term();
        let length = files.len();

        let mut files_iter = files.peekable();
        while let (Some(file), next_file) = (files_iter.next(), files_iter.peek()) {
            let file = Path::new(file);
            if file.is_dir() {
                terminal::disable_raw_mode().unwrap();
                return Err(UUsageError::new(
                    1,
                    format!("{} is a directory.", file.quote()),
                ));
            }
            if !file.exists() {
                terminal::disable_raw_mode().unwrap();
                return Err(USimpleError::new(
                    1,
                    format!("cannot open {}: No such file or directory", file.quote()),
                ));
            }
            if length > 1 {
                buff.push_str(&MULTI_FILE_TOP_PROMPT.replace("{}", file.to_str().unwrap()));
            }
            let mut reader = BufReader::new(File::open(file).unwrap());
            reader.read_to_string(&mut buff).unwrap();
            more(&buff, &mut stdout, next_file.copied(), silent)?;
            buff.clear();
        }
        reset_term(&mut stdout);
    } else if atty::isnt(atty::Stream::Stdin) {
        stdin().read_to_string(&mut buff).unwrap();
        let mut stdout = setup_term();
        more(&buff, &mut stdout, None, silent)?;
        reset_term(&mut stdout);
    } else {
        return Err(UUsageError::new(1, "bad usage"));
    }
    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .about("A file perusal filter for CRT viewing.")
        .version(crate_version!())
        .infer_long_args(true)
        .arg(
            Arg::new(options::SILENT)
                .short('d')
                .long(options::SILENT)
                .help("Display help instead of ringing bell"),
        )
        // The commented arguments below are unimplemented:
        /*
        .arg(
            Arg::new(options::LOGICAL)
                .short('f')
                .long(options::LOGICAL)
                .help("Count logical rather than screen lines"),
        )
        .arg(
            Arg::new(options::NO_PAUSE)
                .short('l')
                .long(options::NO_PAUSE)
                .help("Suppress pause after form feed"),
        )
        .arg(
            Arg::new(options::PRINT_OVER)
                .short('c')
                .long(options::PRINT_OVER)
                .help("Do not scroll, display text and clean line ends"),
        )
        .arg(
            Arg::new(options::CLEAN_PRINT)
                .short('p')
                .long(options::CLEAN_PRINT)
                .help("Do not scroll, clean screen and display text"),
        )
        .arg(
            Arg::new(options::SQUEEZE)
                .short('s')
                .long(options::SQUEEZE)
                .help("Squeeze multiple blank lines into one"),
        )
        .arg(
            Arg::new(options::PLAIN)
                .short('u')
                .long(options::PLAIN)
                .help("Suppress underlining and bold"),
        )
        .arg(
            Arg::new(options::LINES)
                .short('n')
                .long(options::LINES)
                .value_name("number")
                .takes_value(true)
                .help("The number of lines per screen full"),
        )
        .arg(
            Arg::new(options::NUMBER)
                .allow_hyphen_values(true)
                .long(options::NUMBER)
                .required(false)
                .takes_value(true)
                .help("Same as --lines"),
        )
        .arg(
            Arg::new(options::FROM_LINE)
                .short('F')
                .allow_hyphen_values(true)
                .required(false)
                .takes_value(true)
                .value_name("number")
                .help("Display file beginning from line number"),
        )
        .arg(
            Arg::new(options::PATTERN)
                .short('P')
                .allow_hyphen_values(true)
                .required(false)
                .takes_value(true)
                .help("Display file beginning from pattern match"),
        )
        */
        .arg(
            Arg::new(options::FILES)
                .required(false)
                .multiple_occurrences(true)
                .help("Path to the files to be read"),
        )
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

fn more(buff: &str, stdout: &mut Stdout, next_file: Option<&str>, silent: bool) -> UResult<()> {
    let (cols, rows) = terminal::size().unwrap();
    let lines = break_buff(buff, usize::from(cols));

    let mut pager = Pager::new(rows, lines, next_file, silent);
    pager.draw(stdout, None);
    if pager.should_close() {
        return Ok(());
    }

    loop {
        let mut wrong_key = None;
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
                    reset_term(stdout);
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
                    if pager.should_close() {
                        return Ok(());
                    } else {
                        pager.page_down();
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::NONE,
                }) => {
                    pager.page_up();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('j'),
                    modifiers: KeyModifiers::NONE,
                }) => {
                    if pager.should_close() {
                        return Ok(());
                    } else {
                        pager.next_line();
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('k'),
                    modifiers: KeyModifiers::NONE,
                }) => {
                    pager.prev_line();
                }
                Event::Resize(col, row) => {
                    pager.page_resize(col, row);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char(k),
                    ..
                }) => wrong_key = Some(k),
                _ => continue,
            }

            pager.draw(stdout, wrong_key);
        }
    }
}

struct Pager<'a> {
    // The current line at the top of the screen
    upper_mark: usize,
    // The number of rows that fit on the screen
    content_rows: u16,
    lines: Vec<String>,
    next_file: Option<&'a str>,
    line_count: usize,
    silent: bool,
}

impl<'a> Pager<'a> {
    fn new(rows: u16, lines: Vec<String>, next_file: Option<&'a str>, silent: bool) -> Self {
        let line_count = lines.len();
        Self {
            upper_mark: 0,
            content_rows: rows.saturating_sub(1),
            lines,
            next_file,
            line_count,
            silent,
        }
    }

    fn should_close(&mut self) -> bool {
        self.upper_mark
            .saturating_add(self.content_rows.into())
            .ge(&self.line_count)
    }

    fn page_down(&mut self) {
        // If the next page down position __after redraw__ is greater than the total line count,
        // the upper mark must not grow past top of the screen at the end of the open file.
        if self
            .upper_mark
            .saturating_add(self.content_rows as usize * 2)
            .ge(&self.line_count)
        {
            self.upper_mark = self.line_count - self.content_rows as usize;
            return;
        }

        self.upper_mark = self.upper_mark.saturating_add(self.content_rows.into());
    }

    fn page_up(&mut self) {
        self.upper_mark = self.upper_mark.saturating_sub(self.content_rows.into());
    }

    fn next_line(&mut self) {
        self.upper_mark = self.upper_mark.saturating_add(1);
    }

    fn prev_line(&mut self) {
        self.upper_mark = self.upper_mark.saturating_sub(1);
    }

    // TODO: Deal with column size changes.
    fn page_resize(&mut self, _: u16, row: u16) {
        self.content_rows = row.saturating_sub(1);
    }

    fn draw(&self, stdout: &mut std::io::Stdout, wrong_key: Option<char>) {
        let lower_mark = self
            .line_count
            .min(self.upper_mark.saturating_add(self.content_rows.into()));
        self.draw_lines(stdout);
        self.draw_prompt(stdout, lower_mark, wrong_key);
        stdout.flush().unwrap();
    }

    fn draw_lines(&self, stdout: &mut std::io::Stdout) {
        execute!(stdout, terminal::Clear(terminal::ClearType::CurrentLine)).unwrap();
        let displayed_lines = self
            .lines
            .iter()
            .skip(self.upper_mark)
            .take(self.content_rows.into());

        for line in displayed_lines {
            stdout
                .write_all(format!("\r{}\n", line).as_bytes())
                .unwrap();
        }
    }

    fn draw_prompt(&self, stdout: &mut Stdout, lower_mark: usize, wrong_key: Option<char>) {
        let status_inner = if lower_mark == self.line_count {
            format!("Next file: {}", self.next_file.unwrap_or_default())
        } else {
            format!(
                "{}%",
                (lower_mark as f64 / self.line_count as f64 * 100.0).round() as u16
            )
        };

        let status = format!("--More--({})", status_inner);

        let banner = match (self.silent, wrong_key) {
            (true, Some(key)) => format!(
                "{} [Unknown key: '{}'. Press 'h' for instructions. (unimplemented)]",
                status, key
            ),
            (true, None) => format!("{}[Press space to continue, 'q' to quit.]", status),
            (false, Some(_)) => format!("{}{}", status, BELL),
            (false, None) => status,
        };

        write!(
            stdout,
            "\r{}{}{}",
            Attribute::Reverse,
            banner,
            Attribute::Reset
        )
        .unwrap();
    }
}

// Break the lines on the cols of the terminal
fn break_buff(buff: &str, cols: usize) -> Vec<String> {
    let mut lines = Vec::with_capacity(buff.lines().count());

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

#[cfg(test)]
mod tests {
    use super::break_line;
    use unicode_width::UnicodeWidthStr;

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
