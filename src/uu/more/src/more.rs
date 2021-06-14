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
    fs::File,
    io::{stdin, stdout, BufReader, Read, Stdout, Write},
    path::Path,
    time::Duration,
};

#[cfg(all(unix, not(target_os = "fuchsia")))]
extern crate nix;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::Attribute,
    terminal,
};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::app::{get_app, options};

pub mod app;

const BELL: &str = "\x07";

const MULTI_FILE_TOP_PROMPT: &str = "::::::::::::::\n{}\n::::::::::::::\n";

pub fn uumain(args: impl uucore::Args) -> i32 {
    let matches = get_app(executable!()).get_matches_from(args);

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
            more(&buff, &mut stdout, next_file.copied(), silent);
            buff.clear();
        }
        reset_term(&mut stdout);
    } else if atty::isnt(atty::Stream::Stdin) {
        stdin().read_to_string(&mut buff).unwrap();
        let mut stdout = setup_term();
        more(&buff, &mut stdout, None, silent);
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

fn more(buff: &str, mut stdout: &mut Stdout, next_file: Option<&str>, silent: bool) {
    let (cols, rows) = terminal::size().unwrap();
    let lines = break_buff(buff, usize::from(cols));

    let mut pager = Pager::new(rows as usize, lines, next_file, silent);
    pager.draw(stdout, false);
    if pager.should_close() {
        return;
    }

    loop {
        let mut wrong_key = false;
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
                    pager.page_down();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::NONE,
                }) => {
                    pager.page_up();
                }
                _ => {
                    wrong_key = true;
                }
            }

            pager.draw(stdout, wrong_key);
            if pager.should_close() {
                return;
            }
        }
    }
}

struct Pager<'a> {
    // The current line at the top of the screen
    upper_mark: usize,
    // The number of rows that fit on the screen
    content_rows: usize,
    lines: Vec<String>,
    next_file: Option<&'a str>,
    line_count: usize,
    close_on_down: bool,
    silent: bool,
}

impl<'a> Pager<'a> {
    fn new(rows: usize, lines: Vec<String>, next_file: Option<&'a str>, silent: bool) -> Self {
        let line_count = lines.len();
        Self {
            upper_mark: 0,
            content_rows: rows - 1,
            lines,
            next_file,
            line_count,
            close_on_down: false,
            silent,
        }
    }

    fn should_close(&mut self) -> bool {
        if self.upper_mark + self.content_rows >= self.line_count {
            if self.close_on_down {
                return true;
            }
            if self.next_file.is_none() {
                return true;
            } else {
                self.close_on_down = true;
            }
        } else {
            self.close_on_down = false;
        }
        false
    }

    fn page_down(&mut self) {
        self.upper_mark += self.content_rows;
    }

    fn page_up(&mut self) {
        self.upper_mark = self.upper_mark.saturating_sub(self.content_rows);
    }

    fn draw(&self, stdout: &mut std::io::Stdout, wrong_key: bool) {
        let lower_mark = self.line_count.min(self.upper_mark + self.content_rows);
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
            .take(self.content_rows);

        for line in displayed_lines {
            stdout
                .write_all(format!("\r{}\n", line).as_bytes())
                .unwrap();
        }
    }

    fn draw_prompt(&self, stdout: &mut Stdout, lower_mark: usize, wrong_key: bool) {
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
            (true, true) => "[Press 'h' for instructions. (unimplemented)]".to_string(),
            (true, false) => format!("{}[Press space to continue, 'q' to quit.]", status),
            (false, true) => format!("{}{}", status, BELL),
            (false, false) => status,
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
