// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::{
    fs::File,
    io::{BufRead, BufReader, Seek, SeekFrom, Stdin, Stdout, Write, stdin, stdout},
    panic::set_hook,
    path::Path,
    time::Duration,
};

use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use crossterm::{
    cursor::{MoveTo, MoveUp},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::Attribute,
    terminal::{self, Clear, ClearType},
    tty::IsTty,
};

use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::{display::Quotable, show};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("more.md");
const USAGE: &str = help_usage!("more.md");
const BELL: char = '\x07';
const MULTI_FILE_TOP_PROMPT: &str = "\r::::::::::::::\n\r{}\n\r::::::::::::::\n";
const DEFAULT_PROMPT: &str = "--More--";
const HELP_MESSAGE: &str = "[Press space to continue, 'q' to quit.]";

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

struct Options {
    silent: bool,
    _logical: bool,  // not implemented
    _no_pause: bool, // not implemented
    print_over: bool,
    clean_print: bool,
    squeeze: bool,
    lines: Option<u16>,
    from_line: usize,
    pattern: Option<String>,
}

impl Options {
    fn from(matches: &ArgMatches) -> Self {
        let lines = match (
            matches.get_one::<u16>(options::LINES).copied(),
            matches.get_one::<u16>(options::NUMBER).copied(),
        ) {
            // We add 1 to the number of lines to display because the last line
            // is used for the banner
            (Some(n), _) | (None, Some(n)) if n > 0 => Some(n + 1),
            _ => None, // Use terminal height
        };
        let from_line = match matches.get_one::<usize>(options::FROM_LINE).copied() {
            Some(number) => number.saturating_sub(1),
            _ => 0,
        };
        let pattern = matches.get_one::<String>(options::PATTERN).cloned();
        Self {
            silent: matches.get_flag(options::SILENT),
            _logical: matches.get_flag(options::LOGICAL),
            _no_pause: matches.get_flag(options::NO_PAUSE),
            print_over: matches.get_flag(options::PRINT_OVER),
            clean_print: matches.get_flag(options::CLEAN_PRINT),
            squeeze: matches.get_flag(options::SQUEEZE),
            lines,
            from_line,
            pattern,
        }
    }
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
fn reset_term(stdout: &mut Stdout) -> UResult<()> {
    terminal::disable_raw_mode()?;
    // Clear the prompt
    queue!(stdout, Clear(ClearType::CurrentLine))?;
    // Move cursor to the beginning without printing new line
    print!("\r");
    stdout.flush()?;
    Ok(())
}

#[cfg(target_os = "fuchsia")]
#[inline(always)]
fn reset_term(_: &mut usize) -> UResult<()> {
    Ok(())
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Ignore errors in destructor
        let _ = reset_term(&mut stdout());
    }
}

enum InputType {
    File(BufReader<File>),
    Stdin(Stdin),
}

impl InputType {
    fn read_line(&mut self, buf: &mut String) -> std::io::Result<usize> {
        match self {
            InputType::File(reader) => reader.read_line(buf),
            InputType::Stdin(stdin) => stdin.read_line(buf),
        }
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        match self {
            InputType::File(reader) => reader.stream_position(),
            InputType::Stdin(_) => unreachable!("Stdin does not support stream position"),
        }
    }

    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match self {
            InputType::File(reader) => reader.seek(pos),
            InputType::Stdin(_) => unreachable!("Stdin does not support seeking"),
        }
    }

    fn len(&self) -> std::io::Result<Option<u64>> {
        let len = match self {
            InputType::File(reader) => Some(reader.get_ref().metadata()?.len()),
            InputType::Stdin(_) => None,
        };
        Ok(len)
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let _guard = TerminalGuard;

    // Disable raw mode before exiting if a panic occurs
    set_hook(Box::new(|panic_info| {
        // Ignore errors in panic hook
        let _ = terminal::disable_raw_mode();
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
                InputType::File(BufReader::new(opened_file)),
                &mut stdout,
                length > 1,
                file.to_str(),
                next_file.copied(),
                &mut options,
            )?;
        }
    } else {
        let stdin = stdin();
        if stdin.is_tty() {
            // stdin is not a pipe
            return Err(UUsageError::new(1, "bad usage"));
        }
        more(
            InputType::Stdin(stdin),
            &mut stdout,
            false,
            None,
            None,
            &mut options,
        )?;
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
            Arg::new(options::SILENT)
                .short('d')
                .long(options::SILENT)
                .action(ArgAction::SetTrue)
                .help("Display help instead of ringing bell when an illegal key is pressed."),
        )
        .arg(
            Arg::new(options::LOGICAL)
                .short('f')
                .long(options::LOGICAL)
                .action(ArgAction::SetTrue)
                .help("Do not pause after form feed"),
        )
        .arg(
            Arg::new(options::NO_PAUSE)
                .short('l')
                .long(options::NO_PAUSE)
                .action(ArgAction::SetTrue)
                .help("Count logical lines rather than screen lines"),
        )
        .arg(
            Arg::new(options::PRINT_OVER)
                .short('p')
                .long(options::PRINT_OVER)
                .action(ArgAction::SetTrue)
                .help("Do not scroll, clean screen and display text"),
        )
        .arg(
            Arg::new(options::CLEAN_PRINT)
                .short('c')
                .long(options::CLEAN_PRINT)
                .action(ArgAction::SetTrue)
                .help("Do not scroll, display text and clean line ends"),
        )
        .arg(
            Arg::new(options::SQUEEZE)
                .short('s')
                .long(options::SQUEEZE)
                .action(ArgAction::SetTrue)
                .help("Squeeze multiple blank lines into one"),
        )
        .arg(
            Arg::new(options::PLAIN)
                .short('u')
                .long(options::PLAIN)
                .action(ArgAction::SetTrue)
                .hide(true)
                .help("Suppress underlining"),
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
                .help("same as --lines option argument"),
        )
        .arg(
            Arg::new(options::FROM_LINE)
                .short('F')
                .long(options::FROM_LINE)
                .num_args(1)
                .value_name("number")
                .value_parser(value_parser!(usize))
                .help("Start displaying each file at line number"),
        )
        .arg(
            Arg::new(options::PATTERN)
                .short('P')
                .long(options::PATTERN)
                .allow_hyphen_values(true)
                .required(false)
                .value_name("pattern")
                .help("The string to be searched in each file before starting to display it"),
        )
        .arg(
            Arg::new(options::FILES)
                .required(false)
                .action(ArgAction::Append)
                .help("Path to the files to be read")
                .value_hint(clap::ValueHint::FilePath),
        )
}

fn more(
    input: InputType,
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

    let mut pager = Pager::new(input, rows, next_file, options)?;

    if let Some(pattern) = &options.pattern {
        match pager.search_pattern_in_file(pattern)? {
            Some(line) => pager.upper_mark = line,
            None => {
                execute!(stdout, Clear(ClearType::CurrentLine))?;
                stdout.write_all("\rPattern not found\n".as_bytes())?;
                pager.content_rows = pager.content_rows.saturating_sub(1);
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
        pager.content_rows = pager
            .content_rows
            .saturating_sub(MULTI_FILE_TOP_PROMPT.lines().count());
    }

    pager.draw(stdout, None)?;

    if multiple_file {
        pager.content_rows = pager
            .content_rows
            .saturating_add(MULTI_FILE_TOP_PROMPT.lines().count());
        options.from_line = 0;
    }

    if pager.eof_reached && next_file.is_none() {
        return Ok(());
    }

    loop {
        let mut wrong_key = None;
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                // --- Quit ---
                Event::Key(
                    KeyEvent {
                        code: KeyCode::Char('q'),
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

                // --- Forward Navigation ---
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::PageDown | KeyCode::Char(' '),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    if pager.eof_reached {
                        return Ok(());
                    }
                    pager.page_down();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('j'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    if pager.eof_reached {
                        return Ok(());
                    }
                    pager.next_line();
                }

                // --- Backward Navigation (Files Only) ---
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::PageUp,
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    if !pager.is_seekable() {
                        if !options.silent {
                            write!(stdout, "\r{}", BELL)?;
                            stdout.flush()?;
                        }
                        continue;
                    }
                    pager.page_up()?;
                    paging_add_back_message(stdout)?;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('k'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    if !pager.is_seekable() {
                        if !options.silent {
                            write!(stdout, "\r{}", BELL)?;
                            stdout.flush()?;
                        }
                        continue;
                    }
                    pager.prev_line();
                }

                // --- Other Keys ---
                Event::Resize(col, row) => {
                    pager.page_resize(col, row, options.lines);
                }
                Event::Key(KeyEvent {
                    kind: KeyEventKind::Release,
                    ..
                }) => continue,
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

struct Pager<'a> {
    reader: InputType,
    // The current line at the top of the screen
    upper_mark: usize,
    // The number of rows that fit on the screen
    content_rows: usize,
    lines: Vec<String>,
    // Cache of line start byte offsets, where the index corresponds to line number (0-indexed)
    line_offsets: Vec<u64>,
    // Total size of the file in bytes (only for files)
    file_size: Option<u64>,
    next_file: Option<&'a str>,
    eof_reached: bool,
    silent: bool,
    squeeze: bool,
    // Number of lines squeezed out in the current view
    lines_squeezed: usize,
}

impl<'a> Pager<'a> {
    fn new(
        mut input: InputType,
        rows: u16,
        next_file: Option<&'a str>,
        options: &Options,
    ) -> UResult<Self> {
        // Reserve one line for the status bar
        let content_rows = rows.saturating_sub(1).max(1) as usize;

        let file_size = input.len()?;
        let mut line_offsets = vec![0];
        let mut eof_reached = false;

        // Read file up to options.from_line and store line offsets
        let mut line_number = 0;
        let mut byte_position = 0;
        let mut buf = String::new();
        while line_number < options.from_line {
            buf.clear();
            let bytes_read = input.read_line(&mut buf)?;
            if bytes_read == 0 {
                eof_reached = true;
                break;
            }
            byte_position += bytes_read as u64;
            line_offsets.push(byte_position);
            line_number += 1;
        }

        Ok(Self {
            reader: input,
            upper_mark: options.from_line,
            content_rows,
            lines: Vec::with_capacity(content_rows),
            line_offsets,
            file_size,
            next_file,
            eof_reached,
            silent: options.silent,
            squeeze: options.squeeze,
            lines_squeezed: 0,
        })
    }

    fn is_seekable(&self) -> bool {
        matches!(self.reader, InputType::File(_))
    }

    fn page_down(&mut self) {
        if self.eof_reached {
            return;
        }

        self.upper_mark = self.upper_mark.saturating_add(self.content_rows);
    }

    fn next_line(&mut self) {
        if self.eof_reached {
            return;
        }
        // Move the viewing window down by one line
        self.upper_mark = self.upper_mark.saturating_add(1);
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

        self.eof_reached = false;

        Ok(())
    }

    fn prev_line(&mut self) {
        // Move the viewing window up by one line
        self.upper_mark = self.upper_mark.saturating_sub(1);
        self.eof_reached = false;
    }

    // TODO: Deal with column size changes.
    fn page_resize(&mut self, _: u16, row: u16, option_line: Option<u16>) {
        if option_line.is_none() {
            self.content_rows = row.saturating_sub(1) as usize;
        };
    }

    fn draw(&mut self, stdout: &mut Stdout, wrong_key: Option<char>) -> UResult<()> {
        self.draw_lines(stdout)?;
        self.draw_prompt(stdout, wrong_key);
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

    fn draw_prompt(&mut self, stdout: &mut impl Write, wrong_key: Option<char>) {
        let status_inner = match (self.eof_reached, self.next_file) {
            (true, Some(next_file)) => format!("(Next file: {})", next_file),
            _ if self.is_seekable() => match (self.file_size, self.reader.stream_position()) {
                (Some(size), Ok(current_pos)) if size > 0 => {
                    format!(
                        "({}%)",
                        (current_pos as f64 / size as f64 * 100.0).round() as u16
                    )
                }
                _ => String::from("(0%)"),
            },
            _ => String::new(),
        };

        let status = format!("{DEFAULT_PROMPT}{status_inner}");
        let banner = match (self.silent, wrong_key) {
            (true, Some(key)) => format!(
                "{status} [Unknown key: '{key}'. Press 'h' for instructions. (unimplemented)]"
            ),
            (true, None) => format!("{status}{HELP_MESSAGE}"),
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

        if self.is_seekable() {
            self.seek_to_line(self.upper_mark)?;
        }

        let mut line = String::new();
        while self.lines.len() < self.content_rows {
            line.clear();
            if self.reader.read_line(&mut line)? == 0 {
                self.eof_reached = true;
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
        let mut buf = String::new();
        while line_number >= self.line_offsets.len() {
            let last_pos = *self.line_offsets.last().unwrap();
            self.reader.seek(SeekFrom::Start(last_pos))?;

            buf.clear();
            let bytes_read = self.reader.read_line(&mut buf)?;
            if bytes_read == 0 {
                let end_pos = *self.line_offsets.last().unwrap();
                self.reader.seek(SeekFrom::Start(end_pos))?;
                return Ok(());
            }
            self.line_offsets.push(last_pos + bytes_read as u64);
        }

        let pos = self.line_offsets[line_number];
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

    fn search_pattern_in_file(&mut self, pattern: &str) -> UResult<Option<usize>> {
        if pattern.is_empty() {
            return Ok(None);
        }

        let mut line = String::new();
        let mut line_number = self.line_offsets.len();
        let start_line = line_number;
        let mut byte_position = *self.line_offsets.last().unwrap();

        loop {
            line.clear();
            let bytes_read = self.reader.read_line(&mut line)?;
            if bytes_read == 0 {
                self.eof_reached = true;
                break; // EOF
            }

            byte_position += bytes_read as u64;

            if self.line_offsets.len() <= line_number {
                self.line_offsets.push(byte_position);
            }

            if line.contains(pattern) {
                return Ok(Some(line_number));
            }

            line_number += 1;
        }

        if self.is_seekable() {
            self.seek_to_line(start_line)?;
        }
        Ok(None)
    }
}

fn paging_add_back_message(stdout: &mut Stdout) -> UResult<()> {
    execute!(stdout, MoveUp(1))?;
    stdout.write_all("\n\r...back 1 page\n".as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempfile;

    struct TestPagerBuilder {
        content: String,
        options: Options,
        rows: u16,
        next_file: Option<&'static str>,
    }

    impl TestPagerBuilder {
        fn new(content: &str) -> Self {
            Self {
                content: content.to_string(),
                options: Options {
                    silent: false,
                    _logical: false,
                    _no_pause: false,
                    print_over: false,
                    clean_print: false,
                    squeeze: false,
                    lines: None,
                    from_line: 0,
                    pattern: None,
                },
                rows: 24,
                next_file: None,
            }
        }

        fn pattern(mut self, pattern: &str) -> Self {
            self.options.pattern = Some(pattern.to_string());
            self
        }

        fn from_line(mut self, from_line: usize) -> Self {
            self.options.from_line = from_line;
            self
        }

        fn lines(mut self, lines: u16) -> Self {
            self.options.lines = Some(lines);
            self
        }

        fn squeeze(mut self, squeeze: bool) -> Self {
            self.options.squeeze = squeeze;
            self
        }

        fn next_file(mut self, next_file: &'static str) -> Self {
            self.next_file = Some(next_file);
            self
        }

        fn build(self) -> Pager<'static> {
            let mut tmpfile = tempfile().unwrap();
            tmpfile.write_all(self.content.as_bytes()).unwrap();
            tmpfile.rewind().unwrap();
            Pager::new(
                InputType::File(BufReader::new(tmpfile)),
                self.rows,
                self.next_file,
                &self.options,
            )
            .unwrap()
        }
    }

    #[test]
    fn test_pattern_overrides_from_line() {
        let content = "\
line0\n\
line1\n\
line2\n\
PATTERN found here\n\
line4\n";
        let mut pager = TestPagerBuilder::new(content)
            .from_line(3)
            .pattern("PATTERN")
            .build();

        let result = pager.search_pattern_in_file("PATTERN").unwrap();
        assert_eq!(result, Some(3), "Pattern should be found at line index 3");

        if let Some(line) = result {
            pager.upper_mark = line;
        }
        assert_eq!(
            pager.upper_mark, 3,
            "Pattern search should override from_line"
        );
    }

    #[test]
    fn test_from_line_without_pattern() {
        let content = "\
line0\n\
line1\n\
line2\n\
line3\n\
line4\n";
        let pager = TestPagerBuilder::new(content).from_line(4).build();
        assert_eq!(
            pager.upper_mark, 3,
            "Upper mark should be set to from_line - 1"
        );
    }
    #[test]
    fn test_pattern_not_found() {
        let content = "\
line0\n\
line1\n\
line2\n\
line3\n\
line4\n";
        let mut pager = TestPagerBuilder::new(content)
            .pattern("NONEXISTENT")
            .from_line(2)
            .build();

        let result = pager.search_pattern_in_file("NONEXISTENT").unwrap();
        assert_eq!(result, None, "Pattern should not be found");
        assert_eq!(
            pager.upper_mark, 1,
            "Upper mark should remain as the from_line value when pattern is not found"
        );
    }

    #[test]
    fn test_seek_past_end_behavior() {
        let content = "only one line\n";
        let mut pager = TestPagerBuilder::new(content).build();

        let seek_result = pager.seek_to_line(100);
        assert!(
            seek_result.is_ok(),
            "Seeking past end should not error in GNU more behavior"
        );

        pager.lines.clear();
        pager.load_visible_lines().unwrap();
        assert!(
            pager.lines.is_empty(),
            "No visible lines should be loaded when seeking past EOF"
        );
    }

    #[test]
    fn test_squeeze_behavior() {
        let content = "\
line0\n\
\n\
\n\
line1\n\
\n\
line2\n\
\n\
\n\
\n\
line3\n";

        let mut pager = TestPagerBuilder::new(content)
            .squeeze(true)
            .lines(10)
            .build();
        pager.load_visible_lines().unwrap();
        assert_eq!(
            pager.lines.len(),
            7,
            "Squeezed output should collapse consecutive blank lines"
        );
    }

    #[test]
    fn test_next_file_prompt_display() {
        // Prepare a file with minimal content.
        let content = "line1\n";
        let mut pager = TestPagerBuilder::new(content).next_file("next.txt").build();
        // Ensure the pager has read the available content.
        pager.load_visible_lines().unwrap();
        // Simulate reaching EOF.
        pager.eof_reached = true;

        // Capture the prompt output into a buffer.
        let mut output = Vec::new();
        pager.draw_prompt(&mut output, None);

        let output_str = String::from_utf8(output).unwrap();
        // The prompt should include the "Next file: next.txt" message.
        assert!(
            output_str.contains("Next file: next.txt"),
            "Prompt output did not display next file info: {}",
            output_str
        );
    }
}
