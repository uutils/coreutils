// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::{
    fs::File,
    io::{BufRead, BufReader, Cursor, Read, Seek, SeekFrom, Stdout, Write, stdin, stdout},
    panic::set_hook,
    path::Path,
    time::Duration,
};

use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use crossterm::event::KeyEventKind;
use crossterm::{
    cursor::{MoveTo, MoveUp},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::Attribute,
    terminal::{self, Clear, ClearType},
};

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

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        reset_term(&mut stdout());
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let _guard = TerminalGuard;

    // Disable raw mode before exiting if a panic occurs
    set_hook(Box::new(|panic_info| {
        terminal::disable_raw_mode().unwrap();
        print!("\r");
        println!("{panic_info}");
    }));

    let matches = uu_app().try_get_matches_from(args)?;

    let mut options = Options::from(&matches);

    let mut stdout = setup_term()?;

    if let Some(files) = matches.get_many::<String>(options::FILES) {
        let length = files.len();

        let mut files_iter = files.map(|s| s.as_str()).peekable();
        while let (Some(file), next_file) = (files_iter.next(), files_iter.peek()) {
            let file = Path::new(file);
            if file.is_dir() {
                terminal::disable_raw_mode()?;
                show!(UUsageError::new(
                    0,
                    format!("{} is a directory.", file.quote()),
                ));
                terminal::enable_raw_mode()?;
                continue;
            }
            if !file.exists() {
                terminal::disable_raw_mode()?;
                show!(USimpleError::new(
                    0,
                    format!("cannot open {}: No such file or directory", file.quote()),
                ));
                terminal::enable_raw_mode()?;
                continue;
            }
            let opened_file = match File::open(file) {
                Err(why) => {
                    terminal::disable_raw_mode()?;
                    show!(USimpleError::new(
                        0,
                        format!("cannot open {}: {}", file.quote(), why.kind()),
                    ));
                    terminal::enable_raw_mode()?;
                    continue;
                }
                Ok(opened_file) => opened_file,
            };
            more(
                opened_file,
                &mut stdout,
                length > 1,
                file.to_str(),
                next_file.copied(),
                &mut options,
            )?;
        }
    } else {
        let mut buff = String::new();
        stdin().read_to_string(&mut buff)?;
        if buff.is_empty() {
            return Err(UUsageError::new(1, "bad usage"));
        }
        let cursor = Cursor::new(buff);
        more(cursor, &mut stdout, false, None, None, &mut options)?;
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .version(uucore::crate_version!())
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
fn setup_term() -> UResult<Stdout> {
    let stdout = stdout();
    terminal::enable_raw_mode()?;
    Ok(stdout)
}

#[cfg(target_os = "fuchsia")]
#[inline(always)]
fn setup_term() -> UResult<usize> {
    Ok(0)
}

#[cfg(not(target_os = "fuchsia"))]
fn reset_term(stdout: &mut Stdout) {
    terminal::disable_raw_mode().unwrap();
    // Clear the prompt
    queue!(stdout, Clear(ClearType::CurrentLine)).unwrap();
    // Move cursor to the beginning without printing new line
    print!("\r");
    stdout.flush().unwrap();
}

#[cfg(target_os = "fuchsia")]
#[inline(always)]
fn reset_term(_: &mut usize) {}

fn more(
    file: impl Read + Seek + 'static,
    stdout: &mut Stdout,
    multiple_file: bool,
    file_name: Option<&str>,
    next_file: Option<&str>,
    options: &mut Options,
) -> UResult<()> {
    let (_cols, mut rows) = terminal::size()?;
    if let Some(number) = options.lines {
        rows = number;
    }

    let mut pager = Pager::new(file, rows, next_file, options)?;

    if options.pattern.is_some() {
        match pager.pattern_line {
            Some(line) => pager.upper_mark = line,
            None => {
                execute!(stdout, Clear(ClearType::CurrentLine))?;
                stdout.write_all("\rPattern not found\n".as_bytes())?;
                pager.content_rows -= 1;
            }
        }
    }

    if multiple_file {
        execute!(stdout, Clear(ClearType::CurrentLine))?;
        stdout.write_all(
            MULTI_FILE_TOP_PROMPT
                .replace("{}", file_name.unwrap_or_default())
                .as_bytes(),
        )?;
        pager.content_rows -= 3;
    }
    pager.draw(stdout, None)?;
    if multiple_file {
        options.from_line = 0;
        pager.content_rows += 3;
    }

    if pager.should_close() && next_file.is_none() {
        return Ok(());
    }

    loop {
        let mut wrong_key = None;
        if event::poll(Duration::from_millis(10))? {
            match event::read()? {
                Event::Key(KeyEvent {
                    kind: KeyEventKind::Release,
                    ..
                }) => continue,
                Event::Key(
                    KeyEvent {
                        code: KeyCode::Char('q'),
                        modifiers: KeyModifiers::NONE,
                        kind: KeyEventKind::Press,
                        ..
                    }
                    | KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press,
                        ..
                    },
                ) => return Ok(()),
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::PageDown | KeyCode::Char(' '),
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
                    code: KeyCode::Up | KeyCode::PageUp,
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    pager.page_up()?;
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
                execute!(stdout, MoveTo(0, 0), Clear(ClearType::FromCursorDown))?;
            } else if options.clean_print {
                execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
            }
            pager.draw(stdout, wrong_key)?;
        }
    }
}

trait BufReadSeek: BufRead + Seek {}

impl<R: BufRead + Seek> BufReadSeek for R {}

struct Pager<'a> {
    reader: Box<dyn BufReadSeek>,
    // The current line at the top of the screen
    upper_mark: usize,
    // The number of rows that fit on the screen
    content_rows: usize,
    lines: Vec<String>,
    // Cache of line byte positions for faster seeking
    line_positions: Vec<u64>,
    next_file: Option<&'a str>,
    line_count: usize,
    silent: bool,
    squeeze: bool,
    lines_squeezed: usize,
    pattern_line: Option<usize>,
}

impl<'a> Pager<'a> {
    fn new(
        file: impl Read + Seek + 'static,
        rows: u16,
        next_file: Option<&'a str>,
        options: &Options,
    ) -> UResult<Self> {
        // Create buffered reader
        let mut reader = Box::new(BufReader::new(file));

        // Initialize file scanning variables
        let mut line_positions = vec![0]; // Position of first line
        let mut line_count = 0;
        let mut current_position = 0;
        let mut pattern_line = None;
        let mut line = String::new();

        // Scan file to record line positions and find pattern if specified
        loop {
            let bytes = reader.read_line(&mut line)?;
            if bytes == 0 {
                break; // EOF
            }

            line_count += 1;
            current_position += bytes as u64;
            line_positions.push(current_position);

            // Check for pattern match if a pattern was provided
            if pattern_line.is_none() {
                if let Some(ref pattern) = options.pattern {
                    if !pattern.is_empty() && line.contains(pattern) {
                        pattern_line = Some(line_count - 1);
                    }
                }
            }

            line.clear();
        }

        // Reset file position to beginning
        reader.rewind()?;

        // Reserve one line for the status bar
        let content_rows = rows.saturating_sub(1) as usize;

        Ok(Self {
            reader,
            upper_mark: options.from_line,
            content_rows,
            lines: Vec::with_capacity(content_rows),
            line_positions,
            next_file,
            line_count,
            silent: options.silent,
            squeeze: options.squeeze,
            lines_squeezed: 0,
            pattern_line,
        })
    }

    fn should_close(&mut self) -> bool {
        self.upper_mark
            .saturating_add(self.content_rows)
            .ge(&self.line_count)
    }

    fn page_down(&mut self) {
        // If the next page down position __after redraw__ is greater than the total line count,
        // the upper mark must not grow past top of the screen at the end of the open file.
        if self.upper_mark.saturating_add(self.content_rows * 2) >= self.line_count {
            self.upper_mark = self.line_count - self.content_rows;
            return;
        }

        self.upper_mark = self.upper_mark.saturating_add(self.content_rows);
    }

    fn page_up(&mut self) -> UResult<()> {
        self.upper_mark = self
            .upper_mark
            .saturating_sub(self.content_rows.saturating_add(self.lines_squeezed));

        if self.squeeze {
            let mut line = String::new();
            while self.upper_mark > 0 {
                self.seek_to_line(self.upper_mark)?;

                line.clear();
                self.reader.read_line(&mut line)?;

                // Stop if we find a non-empty line
                if line != "\n" {
                    break;
                }

                self.upper_mark = self.upper_mark.saturating_sub(1);
            }
        }

        Ok(())
    }

    fn next_line(&mut self) {
        // Don't proceed if we're already at the last line
        if self.upper_mark >= self.line_count.saturating_sub(1) {
            return;
        }

        // Move the viewing window down by one line
        self.upper_mark = self.upper_mark.saturating_add(1);
    }

    fn prev_line(&mut self) {
        // Don't proceed if we're already at the first line
        if self.upper_mark == 0 {
            return;
        }

        // Move the viewing window up by one line
        self.upper_mark = self.upper_mark.saturating_sub(1);
    }

    // TODO: Deal with column size changes.
    fn page_resize(&mut self, _: u16, row: u16, option_line: Option<u16>) {
        if option_line.is_none() {
            self.content_rows = row.saturating_sub(1) as usize;
        };
    }

    fn draw(&mut self, stdout: &mut Stdout, wrong_key: Option<char>) -> UResult<()> {
        self.draw_lines(stdout)?;
        let lower_mark = self
            .line_count
            .min(self.upper_mark.saturating_add(self.content_rows));
        self.draw_prompt(stdout, lower_mark, wrong_key);
        stdout.flush()?;
        Ok(())
    }

    fn draw_lines(&mut self, stdout: &mut Stdout) -> UResult<()> {
        execute!(stdout, Clear(ClearType::CurrentLine))?;

        self.load_visible_lines()?;
        for line in &self.lines {
            stdout.write_all(format!("\r{line}").as_bytes())?;
        }
        Ok(())
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
            "\r{}{banner}{}",
            Attribute::Reverse,
            Attribute::Reset
        )
        .unwrap();
    }

    fn load_visible_lines(&mut self) -> UResult<()> {
        self.lines.clear();

        self.lines_squeezed = 0;

        self.seek_to_line(self.upper_mark)?;

        let mut line = String::new();
        while self.lines.len() < self.content_rows {
            line.clear();
            if self.reader.read_line(&mut line)? == 0 {
                break; // EOF
            }

            if self.should_squeeze_line(&line) {
                self.lines_squeezed += 1;
            } else {
                self.lines.push(std::mem::take(&mut line));
            }
        }

        Ok(())
    }

    fn seek_to_line(&mut self, line_number: usize) -> UResult<()> {
        let line_number = line_number.min(self.line_count);
        let pos = self.line_positions[line_number];
        self.reader.seek(SeekFrom::Start(pos))?;
        Ok(())
    }

    fn should_squeeze_line(&self, line: &str) -> bool {
        if !self.squeeze {
            return false;
        }

        let is_empty = line.trim().is_empty();
        let prev_empty = self
            .lines
            .last()
            .map(|l| l.trim().is_empty())
            .unwrap_or(false);

        is_empty && prev_empty
    }
}

fn paging_add_back_message(options: &Options, stdout: &mut Stdout) -> UResult<()> {
    if options.lines.is_some() {
        execute!(stdout, MoveUp(1))?;
        stdout.write_all("\n\r...back 1 page\n".as_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPagerBuilder {
        content: String,
        options: Options,
        rows: u16,
        next_file: Option<&'static str>,
    }

    #[allow(dead_code)]
    impl TestPagerBuilder {
        fn new(content: &str) -> Self {
            Self {
                content: content.to_string(),
                options: Options {
                    clean_print: false,
                    from_line: 0,
                    lines: None,
                    pattern: None,
                    print_over: false,
                    silent: false,
                    squeeze: false,
                },
                rows: 24,
                next_file: None,
            }
        }

        fn build(self) -> Pager<'static> {
            let cursor = Cursor::new(self.content);
            Pager::new(cursor, self.rows, self.next_file, &self.options).unwrap()
        }

        fn pattern(mut self, pattern: &str) -> Self {
            self.options.pattern = Some(pattern.to_owned());
            self
        }

        fn clean_print(mut self, clean_print: bool) -> Self {
            self.options.clean_print = clean_print;
            self
        }

        #[allow(clippy::wrong_self_convention)]
        fn from_line(mut self, from_line: usize) -> Self {
            self.options.from_line = from_line;
            self
        }

        fn lines(mut self, lines: u16) -> Self {
            self.options.lines = Some(lines);
            self
        }

        fn print_over(mut self, print_over: bool) -> Self {
            self.options.print_over = print_over;
            self
        }

        fn silent(mut self, silent: bool) -> Self {
            self.options.silent = silent;
            self
        }

        fn squeeze(mut self, squeeze: bool) -> Self {
            self.options.squeeze = squeeze;
            self
        }

        fn rows(mut self, rows: u16) -> Self {
            self.rows = rows;
            self
        }

        fn next_file(mut self, next_file: &'static str) -> Self {
            self.next_file = Some(next_file);
            self
        }
    }

    mod pattern_search {
        use super::*;

        #[test]
        fn test_empty_file() {
            let pager = TestPagerBuilder::new("").pattern("pattern").build();
            assert_eq!(None, pager.pattern_line);
        }

        #[test]
        fn test_empty_pattern() {
            let pager = TestPagerBuilder::new("line1\nline2\nline3\n")
                .pattern("")
                .build();
            assert_eq!(None, pager.pattern_line);
        }

        #[test]
        fn test_pattern_found() {
            let pager = TestPagerBuilder::new("line1\nline2\npattern\n")
                .pattern("pattern")
                .build();
            assert_eq!(Some(2), pager.pattern_line);

            let pager = TestPagerBuilder::new("line1\nline2\npattern\npattern2\n")
                .pattern("pattern")
                .build();
            assert_eq!(Some(2), pager.pattern_line);

            let pager = TestPagerBuilder::new("line1\nline2\nother_pattern\n")
                .pattern("pattern")
                .build();
            assert_eq!(Some(2), pager.pattern_line);
        }

        #[test]
        fn test_pattern_not_found() {
            let pager = TestPagerBuilder::new("line1\nline2\nsomething\n")
                .pattern("pattern")
                .build();
            assert_eq!(None, pager.pattern_line);
        }
    }

    mod pager_initialization {
        use super::*;

        #[test]
        fn test_init_preserves_position() {
            let mut pager = TestPagerBuilder::new("line1\nline2\npattern\n")
                .pattern("pattern")
                .build();
            assert_eq!(Some(2), pager.pattern_line);
            assert_eq!(0, pager.reader.stream_position().unwrap());
        }
    }

    mod seeking {
        use super::*;

        #[test]
        fn test_seek_past_end() {
            let mut pager = TestPagerBuilder::new("just one line").build();
            assert!(pager.seek_to_line(100).is_ok());
        }

        #[test]
        fn test_seek_in_empty_file() {
            let mut empty_pager = TestPagerBuilder::new("").build();
            assert!(empty_pager.seek_to_line(5).is_ok());
        }
    }
}
