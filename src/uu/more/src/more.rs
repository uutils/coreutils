// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::{
    fs::File,
    io::{stdin, stdout, BufReader, Read, Stdout, Write},
    panic::set_hook,
    path::Path,
    time::Duration,
};

use clap::{crate_version, value_parser, Arg, ArgAction, ArgMatches, Command};
use crossterm::event::KeyEventKind;
use crossterm::{
    cursor::{MoveTo, MoveUp},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::Attribute,
    terminal::{self, Clear, ClearType},
};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::{display::Quotable, show};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("more.md");
const USAGE: &str = help_usage!("more.md");
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

const MULTI_FILE_TOP_PROMPT: &str = "\r::::::::::::::\n\r{}\n\r::::::::::::::\n";

struct Options {
    clean_print: bool,
    from_line: usize,
    lines: Option<u16>,
    pattern: Option<String>,
    print_over: bool,
    silent: bool,
    squeeze: bool,
}

impl Options {
    fn from(matches: &ArgMatches) -> Self {
        let lines = match (
            matches.get_one::<u16>(options::LINES).copied(),
            matches.get_one::<u16>(options::NUMBER).copied(),
        ) {
            // We add 1 to the number of lines to display because the last line
            // is used for the banner
            (Some(number), _) if number > 0 => Some(number + 1),
            (None, Some(number)) if number > 0 => Some(number + 1),
            (_, _) => None,
        };
        let from_line = match matches.get_one::<usize>(options::FROM_LINE).copied() {
            Some(number) if number > 1 => number - 1,
            _ => 0,
        };
        let pattern = matches
            .get_one::<String>(options::PATTERN)
            .map(|s| s.to_owned());
        Self {
            clean_print: matches.get_flag(options::CLEAN_PRINT),
            from_line,
            lines,
            pattern,
            print_over: matches.get_flag(options::PRINT_OVER),
            silent: matches.get_flag(options::SILENT),
            squeeze: matches.get_flag(options::SQUEEZE),
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    // Disable raw mode before exiting if a panic occurs
    set_hook(Box::new(|panic_info| {
        terminal::disable_raw_mode().unwrap();
        print!("\r");
        println!("{panic_info}");
    }));

    let matches = match uu_app().try_get_matches_from(args) {
        Ok(m) => m,
        Err(e) => return Err(e.into()),
    };

    let mut options = Options::from(&matches);

    let mut buff = String::new();

    if let Some(files) = matches.get_many::<String>(options::FILES) {
        let mut stdout = setup_term();
        let length = files.len();

        let mut files_iter = files.map(|s| s.as_str()).peekable();
        while let (Some(file), next_file) = (files_iter.next(), files_iter.peek()) {
            let file = Path::new(file);
            if file.is_dir() {
                terminal::disable_raw_mode().unwrap();
                show!(UUsageError::new(
                    0,
                    format!("{} is a directory.", file.quote()),
                ));
                terminal::enable_raw_mode().unwrap();
                continue;
            }
            if !file.exists() {
                terminal::disable_raw_mode().unwrap();
                show!(USimpleError::new(
                    0,
                    format!("cannot open {}: No such file or directory", file.quote()),
                ));
                terminal::enable_raw_mode().unwrap();
                continue;
            }
            let opened_file = match File::open(file) {
                Err(why) => {
                    terminal::disable_raw_mode().unwrap();
                    show!(USimpleError::new(
                        0,
                        format!("cannot open {}: {}", file.quote(), why.kind()),
                    ));
                    terminal::enable_raw_mode().unwrap();
                    continue;
                }
                Ok(opened_file) => opened_file,
            };
            let mut reader = BufReader::new(opened_file);
            reader.read_to_string(&mut buff).unwrap();
            more(
                &buff,
                &mut stdout,
                length > 1,
                file.to_str(),
                next_file.copied(),
                &mut options,
            )?;
            buff.clear();
        }
        reset_term(&mut stdout);
    } else {
        stdin().read_to_string(&mut buff).unwrap();
        if buff.is_empty() {
            return Err(UUsageError::new(1, "bad usage"));
        }
        let mut stdout = setup_term();
        more(&buff, &mut stdout, false, None, None, &mut options)?;
        reset_term(&mut stdout);
    }
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .version(crate_version!())
        .infer_long_args(true)
        .arg(
            Arg::new(options::PRINT_OVER)
                .short('c')
                .long(options::PRINT_OVER)
                .help("Do not scroll, display text and clean line ends")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SILENT)
                .short('d')
                .long(options::SILENT)
                .help("Display help instead of ringing bell")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CLEAN_PRINT)
                .short('p')
                .long(options::CLEAN_PRINT)
                .help("Do not scroll, clean screen and display text")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SQUEEZE)
                .short('s')
                .long(options::SQUEEZE)
                .help("Squeeze multiple blank lines into one")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PLAIN)
                .short('u')
                .long(options::PLAIN)
                .action(ArgAction::SetTrue)
                .hide(true),
        )
        .arg(
            Arg::new(options::PATTERN)
                .short('P')
                .long(options::PATTERN)
                .allow_hyphen_values(true)
                .required(false)
                .value_name("pattern")
                .help("Display file beginning from pattern match"),
        )
        .arg(
            Arg::new(options::FROM_LINE)
                .short('F')
                .long(options::FROM_LINE)
                .num_args(1)
                .value_name("number")
                .value_parser(value_parser!(usize))
                .help("Display file beginning from line number"),
        )
        .arg(
            Arg::new(options::LINES)
                .short('n')
                .long(options::LINES)
                .value_name("number")
                .num_args(1)
                .value_parser(value_parser!(u16).range(0..))
                .help("The number of lines per screen full"),
        )
        .arg(
            Arg::new(options::NUMBER)
                .long(options::NUMBER)
                .num_args(1)
                .value_parser(value_parser!(u16).range(0..))
                .help("Same as --lines"),
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
        */
        .arg(
            Arg::new(options::FILES)
                .required(false)
                .action(ArgAction::Append)
                .help("Path to the files to be read")
                .value_hint(clap::ValueHint::FilePath),
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
    queue!(stdout, terminal::Clear(ClearType::CurrentLine)).unwrap();
    // Move cursor to the beginning without printing new line
    print!("\r");
    stdout.flush().unwrap();
}

#[cfg(target_os = "fuchsia")]
#[inline(always)]
fn reset_term(_: &mut usize) {}

fn more(
    buff: &str,
    stdout: &mut Stdout,
    multiple_file: bool,
    file: Option<&str>,
    next_file: Option<&str>,
    options: &mut Options,
) -> UResult<()> {
    let (cols, mut rows) = terminal::size().unwrap();
    if let Some(number) = options.lines {
        rows = number;
    }

    let lines = break_buff(buff, usize::from(cols));

    let mut pager = Pager::new(rows, lines, next_file, options);

    if options.pattern.is_some() {
        match search_pattern_in_file(&pager.lines, &options.pattern) {
            Some(number) => pager.upper_mark = number,
            None => {
                execute!(stdout, terminal::Clear(terminal::ClearType::CurrentLine))?;
                stdout.write_all("\rPattern not found\n".as_bytes())?;
                pager.content_rows -= 1;
            }
        }
    }

    if multiple_file {
        execute!(stdout, terminal::Clear(terminal::ClearType::CurrentLine)).unwrap();
        stdout.write_all(
            MULTI_FILE_TOP_PROMPT
                .replace("{}", file.unwrap_or_default())
                .as_bytes(),
        )?;
        pager.content_rows -= 3;
    }
    pager.draw(stdout, None);
    if multiple_file {
        options.from_line = 0;
        pager.content_rows += 3;
    }

    if pager.should_close() && next_file.is_none() {
        return Ok(());
    }

    loop {
        let mut wrong_key = None;
        if event::poll(Duration::from_millis(10)).unwrap() {
            match event::read().unwrap() {
                Event::Key(KeyEvent {
                    kind: KeyEventKind::Release,
                    ..
                }) => continue,
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press,
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    reset_term(stdout);
                    std::process::exit(0);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    modifiers: KeyModifiers::NONE,
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::PageDown,
                    modifiers: KeyModifiers::NONE,
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char(' '),
                    modifiers: KeyModifiers::NONE,
                    ..
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
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::PageUp,
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    pager.page_up();
                    paging_add_back_message(options, stdout)?;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('j'),
                    modifiers: KeyModifiers::NONE,
                    ..
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
                    ..
                }) => {
                    pager.prev_line();
                }
                Event::Resize(col, row) => {
                    pager.page_resize(col, row, options.lines);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char(k),
                    ..
                }) => wrong_key = Some(k),
                _ => continue,
            }

            if options.print_over {
                execute!(
                    std::io::stdout(),
                    MoveTo(0, 0),
                    Clear(ClearType::FromCursorDown)
                )
                .unwrap();
            } else if options.clean_print {
                execute!(std::io::stdout(), Clear(ClearType::All), MoveTo(0, 0)).unwrap();
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
    squeeze: bool,
    line_squeezed: usize,
}

impl<'a> Pager<'a> {
    fn new(rows: u16, lines: Vec<String>, next_file: Option<&'a str>, options: &Options) -> Self {
        let line_count = lines.len();
        Self {
            upper_mark: options.from_line,
            content_rows: rows.saturating_sub(1),
            lines,
            next_file,
            line_count,
            silent: options.silent,
            squeeze: options.squeeze,
            line_squeezed: 0,
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
        let content_row_usize: usize = self.content_rows.into();
        self.upper_mark = self
            .upper_mark
            .saturating_sub(content_row_usize.saturating_add(self.line_squeezed));

        if self.squeeze {
            let iter = self.lines.iter().take(self.upper_mark).rev();
            for line in iter {
                if line.is_empty() {
                    self.upper_mark = self.upper_mark.saturating_sub(1);
                } else {
                    break;
                }
            }
        }
    }

    fn next_line(&mut self) {
        self.upper_mark = self.upper_mark.saturating_add(1);
    }

    fn prev_line(&mut self) {
        self.upper_mark = self.upper_mark.saturating_sub(1);
    }

    // TODO: Deal with column size changes.
    fn page_resize(&mut self, _: u16, row: u16, option_line: Option<u16>) {
        if option_line.is_none() {
            self.content_rows = row.saturating_sub(1);
        };
    }

    fn draw(&mut self, stdout: &mut std::io::Stdout, wrong_key: Option<char>) {
        self.draw_lines(stdout);
        let lower_mark = self
            .line_count
            .min(self.upper_mark.saturating_add(self.content_rows.into()));
        self.draw_prompt(stdout, lower_mark, wrong_key);
        stdout.flush().unwrap();
    }

    fn draw_lines(&mut self, stdout: &mut std::io::Stdout) {
        execute!(stdout, terminal::Clear(terminal::ClearType::CurrentLine)).unwrap();

        self.line_squeezed = 0;
        let mut previous_line_blank = false;
        let mut displayed_lines = Vec::new();
        let mut iter = self.lines.iter().skip(self.upper_mark);

        while displayed_lines.len() < self.content_rows as usize {
            match iter.next() {
                Some(line) => {
                    if self.squeeze {
                        match (line.is_empty(), previous_line_blank) {
                            (true, false) => {
                                previous_line_blank = true;
                                displayed_lines.push(line);
                            }
                            (false, true) => {
                                previous_line_blank = false;
                                displayed_lines.push(line);
                            }
                            (false, false) => displayed_lines.push(line),
                            (true, true) => {
                                self.line_squeezed += 1;
                                self.upper_mark += 1;
                            }
                        }
                    } else {
                        displayed_lines.push(line);
                    }
                }
                // if none the end of the file is reached
                None => {
                    self.upper_mark = self.line_count;
                    break;
                }
            }
        }

        for line in displayed_lines {
            stdout.write_all(format!("\r{line}\n").as_bytes()).unwrap();
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

        let status = format!("--More--({status_inner})");
        let banner = match (self.silent, wrong_key) {
            (true, Some(key)) => format!(
                "{status} [Unknown key: '{key}'. Press 'h' for instructions. (unimplemented)]"
            ),
            (true, None) => format!("{status}[Press space to continue, 'q' to quit.]"),
            (false, Some(_)) => format!("{status}{BELL}"),
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

fn search_pattern_in_file(lines: &[String], pattern: &Option<String>) -> Option<usize> {
    let pattern = pattern.clone().unwrap_or_default();
    if lines.is_empty() || pattern.is_empty() {
        return None;
    }
    for (line_number, line) in lines.iter().enumerate() {
        if line.contains(pattern.as_str()) {
            return Some(line_number);
        }
    }
    None
}

fn paging_add_back_message(options: &Options, stdout: &mut std::io::Stdout) -> UResult<()> {
    if options.lines.is_some() {
        execute!(stdout, MoveUp(1))?;
        stdout.write_all("\n\r...back 1 page\n".as_bytes())?;
    }
    Ok(())
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
    use super::{break_line, search_pattern_in_file};
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

    #[test]
    fn test_search_pattern_empty_lines() {
        let lines = vec![];
        let pattern = Some(String::from("pattern"));
        assert_eq!(None, search_pattern_in_file(&lines, &pattern));
    }

    #[test]
    fn test_search_pattern_empty_pattern() {
        let lines = vec![String::from("line1"), String::from("line2")];
        let pattern = None;
        assert_eq!(None, search_pattern_in_file(&lines, &pattern));
    }

    #[test]
    fn test_search_pattern_found_pattern() {
        let lines = vec![
            String::from("line1"),
            String::from("line2"),
            String::from("pattern"),
        ];
        let lines2 = vec![
            String::from("line1"),
            String::from("line2"),
            String::from("pattern"),
            String::from("pattern2"),
        ];
        let lines3 = vec![
            String::from("line1"),
            String::from("line2"),
            String::from("other_pattern"),
        ];
        let pattern = Some(String::from("pattern"));
        assert_eq!(2, search_pattern_in_file(&lines, &pattern).unwrap());
        assert_eq!(2, search_pattern_in_file(&lines2, &pattern).unwrap());
        assert_eq!(2, search_pattern_in_file(&lines3, &pattern).unwrap());
    }

    #[test]
    fn test_search_pattern_not_found_pattern() {
        let lines = vec![
            String::from("line1"),
            String::from("line2"),
            String::from("something"),
        ];
        let pattern = Some(String::from("pattern"));
        assert_eq!(None, search_pattern_in_file(&lines, &pattern));
    }
}
