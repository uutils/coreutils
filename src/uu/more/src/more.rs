// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::{
    ffi::OsString,
    fs::File,
    io::{BufRead, BufReader, Stdin, Stdout, Write, stdin, stdout},
    panic::set_hook,
    path::{Path, PathBuf},
    time::Duration,
};

use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use crossterm::{
    ExecutableCommand,
    QueueableCommand, // spell-checker:disable-line
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::Attribute,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
};

use uucore::error::{UResult, USimpleError, UUsageError};
use uucore::format_usage;
use uucore::{display::Quotable, show};

use uucore::translate;

#[derive(Debug)]
enum MoreError {
    IsDirectory(PathBuf),
    CannotOpenNoSuchFile(PathBuf),
    CannotOpenIOError(PathBuf, std::io::ErrorKind),
    BadUsage,
}

impl std::fmt::Display for MoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IsDirectory(path) => {
                write!(
                    f,
                    "{}",
                    translate!(
                        "more-error-is-directory",
                        "path" => path.quote()
                    )
                )
            }
            Self::CannotOpenNoSuchFile(path) => {
                write!(
                    f,
                    "{}",
                    translate!(
                        "more-error-cannot-open-no-such-file",
                        "path" => path.quote()
                    )
                )
            }
            Self::CannotOpenIOError(path, error) => {
                write!(
                    f,
                    "{}",
                    translate!(
                    "more-error-cannot-open-io-error",
                    "path" => path.quote(),
                    "error" => error
                    )
                )
            }
            Self::BadUsage => {
                write!(f, "{}", translate!("more-error-bad-usage"))
            }
        }
    }
}

impl std::error::Error for MoreError {}

const BELL: char = '\x07'; // Printing this character will ring the bell

// The prompt to be displayed at the top of the screen when viewing multiple files,
// with the file name in the middle
const MULTI_FILE_TOP_PROMPT: &str = "\r::::::::::::::\n\r{}\n\r::::::::::::::\n";

pub mod options {
    pub const SILENT: &str = "silent";
    pub const LOGICAL: &str = "logical";
    pub const EXIT_ON_EOF: &str = "exit-on-eof";
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
    _logical: bool,     // not implemented
    _exit_on_eof: bool, // not implemented
    _no_pause: bool,    // not implemented
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
            _exit_on_eof: matches.get_flag(options::EXIT_ON_EOF),
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

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    set_hook(Box::new(|panic_info| {
        print!("\r");
        println!("{panic_info}");
    }));
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;
    let mut options = Options::from(&matches);
    if let Some(files) = matches.get_many::<OsString>(options::FILES) {
        let length = files.len();

        let mut files_iter = files.peekable();
        while let (Some(file_os), next_file) = (files_iter.next(), files_iter.peek()) {
            let file = Path::new(file_os);
            if file.is_dir() {
                show!(UUsageError::new(
                    0,
                    MoreError::IsDirectory(file.into()).to_string(),
                ));
                continue;
            }
            if !file.exists() {
                show!(USimpleError::new(
                    0,
                    MoreError::CannotOpenNoSuchFile(file.into()).to_string(),
                ));
                continue;
            }
            let opened_file = match File::open(file) {
                Err(why) => {
                    show!(USimpleError::new(
                        0,
                        MoreError::CannotOpenIOError(file.into(), why.kind()).to_string(),
                    ));
                    continue;
                }
                Ok(opened_file) => opened_file,
            };
            let next_file_str = next_file.map(|f| f.to_string_lossy().into_owned());
            more(
                InputType::File(BufReader::new(opened_file)),
                length > 1,
                Some(&file.to_string_lossy()),
                next_file_str.as_deref(),
                &mut options,
            )?;
        }
    } else {
        let stdin = stdin();
        if stdin.is_tty() {
            // stdin is not a pipe
            return Err(UUsageError::new(1, MoreError::BadUsage.to_string()));
        }
        more(InputType::Stdin(stdin), false, None, None, &mut options)?;
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(translate!("more-about"))
        .override_usage(format_usage(&translate!("more-usage")))
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .infer_long_args(true)
        .arg(
            Arg::new(options::SILENT)
                .short('d')
                .long(options::SILENT)
                .action(ArgAction::SetTrue)
                .help(translate!("more-help-silent")),
        )
        .arg(
            Arg::new(options::LOGICAL)
                .short('l')
                .long(options::LOGICAL)
                .action(ArgAction::SetTrue)
                .help(translate!("more-help-logical")),
        )
        .arg(
            Arg::new(options::EXIT_ON_EOF)
                .short('e')
                .long(options::EXIT_ON_EOF)
                .action(ArgAction::SetTrue)
                .help(translate!("more-help-exit-on-eof")),
        )
        .arg(
            Arg::new(options::NO_PAUSE)
                .short('f')
                .long(options::NO_PAUSE)
                .action(ArgAction::SetTrue)
                .help(translate!("more-help-no-pause")),
        )
        .arg(
            Arg::new(options::PRINT_OVER)
                .short('p')
                .long(options::PRINT_OVER)
                .action(ArgAction::SetTrue)
                .help(translate!("more-help-print-over")),
        )
        .arg(
            Arg::new(options::CLEAN_PRINT)
                .short('c')
                .long(options::CLEAN_PRINT)
                .action(ArgAction::SetTrue)
                .help(translate!("more-help-clean-print")),
        )
        .arg(
            Arg::new(options::SQUEEZE)
                .short('s')
                .long(options::SQUEEZE)
                .action(ArgAction::SetTrue)
                .help(translate!("more-help-squeeze")),
        )
        .arg(
            Arg::new(options::PLAIN)
                .short('u')
                .long(options::PLAIN)
                .action(ArgAction::SetTrue)
                .hide(true)
                .help(translate!("more-help-plain")),
        )
        .arg(
            Arg::new(options::LINES)
                .short('n')
                .long(options::LINES)
                .value_name("number")
                .num_args(1)
                .value_parser(value_parser!(u16).range(0..))
                .help(translate!("more-help-lines")),
        )
        .arg(
            Arg::new(options::NUMBER)
                .long(options::NUMBER)
                .num_args(1)
                .value_parser(value_parser!(u16).range(0..))
                .help(translate!("more-help-number")),
        )
        .arg(
            Arg::new(options::FROM_LINE)
                .short('F')
                .long(options::FROM_LINE)
                .num_args(1)
                .value_name("number")
                .value_parser(value_parser!(usize))
                .help(translate!("more-help-from-line")),
        )
        .arg(
            Arg::new(options::PATTERN)
                .short('P')
                .long(options::PATTERN)
                .allow_hyphen_values(true)
                .required(false)
                .value_name("pattern")
                .help(translate!("more-help-pattern")),
        )
        .arg(
            Arg::new(options::FILES)
                .required(false)
                .action(ArgAction::Append)
                .help(translate!("more-help-files"))
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
}

enum InputType {
    File(BufReader<File>),
    Stdin(Stdin),
}

impl InputType {
    fn read_line(&mut self, buf: &mut String) -> std::io::Result<usize> {
        match self {
            Self::File(reader) => reader.read_line(buf),
            Self::Stdin(stdin) => stdin.read_line(buf),
        }
    }

    fn len(&self) -> std::io::Result<Option<u64>> {
        let len = match self {
            Self::File(reader) => Some(reader.get_ref().metadata()?.len()),
            Self::Stdin(_) => None,
        };
        Ok(len)
    }
}

enum OutputType {
    Tty(Stdout),
    Pipe(Box<dyn Write>),
    #[cfg(test)]
    Test(Vec<u8>),
}

impl IsTty for OutputType {
    fn is_tty(&self) -> bool {
        matches!(self, Self::Tty(_))
    }
}

impl Write for OutputType {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Tty(stdout) => stdout.write(buf),
            Self::Pipe(writer) => writer.write(buf),
            #[cfg(test)]
            Self::Test(vec) => vec.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Tty(stdout) => stdout.flush(),
            Self::Pipe(writer) => writer.flush(),
            #[cfg(test)]
            Self::Test(vec) => vec.flush(),
        }
    }
}

fn setup_term() -> UResult<OutputType> {
    let mut stdout = stdout();
    if stdout.is_tty() {
        terminal::enable_raw_mode()?;
        stdout.execute(EnterAlternateScreen)?.execute(Hide)?;
        Ok(OutputType::Tty(stdout))
    } else {
        Ok(OutputType::Pipe(Box::new(stdout)))
    }
}

#[cfg(target_os = "fuchsia")]
#[inline(always)]
fn setup_term() -> UResult<OutputType> {
    // no real stdout/tty on Fuchsia, just write into a pipe
    Ok(OutputType::Pipe(Box::new(stdout())))
}

fn reset_term() -> UResult<()> {
    let mut stdout = stdout();
    if stdout.is_tty() {
        stdout.queue(Show)?.queue(LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
    } else {
        stdout.queue(Clear(ClearType::CurrentLine))?;
        write!(stdout, "\r")?;
    }
    stdout.flush()?;
    Ok(())
}

#[cfg(target_os = "fuchsia")]
#[inline(always)]
fn reset_term() -> UResult<()> {
    Ok(())
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Ignore errors in destructor
        let _ = reset_term();
    }
}

fn more(
    input: InputType,
    multiple_file: bool,
    file_name: Option<&str>,
    next_file: Option<&str>,
    options: &mut Options,
) -> UResult<()> {
    // Initialize output
    let out = setup_term()?;
    // Ensure raw mode is disabled on drop
    let _guard = TerminalGuard;
    // Create pager
    let (_cols, mut rows) = terminal::size()?;
    if let Some(number) = options.lines {
        rows = number;
    }
    let mut pager = Pager::new(input, rows, file_name, next_file, options, out)?;
    // Start from the specified line
    pager.handle_from_line()?;
    // Search for pattern
    pager.handle_pattern_search()?;
    // Handle multi-file display header if needed
    if multiple_file {
        pager.display_multi_file_header()?;
    }
    // Initial display
    pager.draw(None)?;
    // Reset multi-file settings after initial display
    if multiple_file {
        pager.reset_multi_file_header();
        options.from_line = 0;
    }
    // Main event loop
    pager.process_events(options)
}

struct Pager<'a> {
    /// Source of the content (file, stdin)
    input: InputType,
    /// Total size of the file in bytes (only available for file inputs)
    file_size: Option<u64>,
    /// Storage for the lines read from the input
    lines: Vec<String>,
    /// Running total of byte sizes for each line, used for positioning
    cumulative_line_sizes: Vec<u64>,
    /// Index of the line currently displayed at the top of the screen
    upper_mark: usize,
    /// Number of rows that can be displayed on the screen at once
    content_rows: usize,
    /// Count of blank lines that have been condensed in the current view
    lines_squeezed: usize,
    pattern: Option<String>,
    file_name: Option<&'a str>,
    next_file: Option<&'a str>,
    eof_reached: bool,
    silent: bool,
    squeeze: bool,
    stdout: OutputType,
}

impl<'a> Pager<'a> {
    fn new(
        input: InputType,
        rows: u16,
        file_name: Option<&'a str>,
        next_file: Option<&'a str>,
        options: &Options,
        stdout: OutputType,
    ) -> UResult<Self> {
        // Reserve one line for the status bar, ensuring at least one content row
        let content_rows = rows.saturating_sub(1).max(1) as usize;
        let file_size = input.len()?;
        let pager = Self {
            input,
            file_size,
            lines: Vec::with_capacity(content_rows),
            cumulative_line_sizes: Vec::new(),
            upper_mark: options.from_line,
            content_rows,
            lines_squeezed: 0,
            pattern: options.pattern.clone(),
            file_name,
            next_file,
            eof_reached: false,
            silent: options.silent,
            squeeze: options.squeeze,
            stdout,
        };
        Ok(pager)
    }

    fn handle_from_line(&mut self) -> UResult<()> {
        if !self.read_until_line(self.upper_mark)? {
            write!(
                self.stdout,
                "\r{}{} ({}){}",
                Attribute::Reverse,
                translate!(
                    "more-error-cannot-seek-to-line",
                    "line" => (self.upper_mark + 1)
                ),
                translate!("more-press-return"),
                Attribute::Reset,
            )?;
            self.stdout.flush()?;
            self.wait_for_enter_key()?;
            self.upper_mark = 0;
        }
        Ok(())
    }

    fn read_until_line(&mut self, target_line: usize) -> UResult<bool> {
        // Read lines until we reach the target line or EOF
        let mut line = String::new();
        while self.lines.len() <= target_line {
            let bytes_read = self.input.read_line(&mut line)?;
            if bytes_read == 0 {
                return Ok(false); // EOF
            }
            // Track cumulative byte position
            let last_pos = self.cumulative_line_sizes.last().copied().unwrap_or(0);
            self.cumulative_line_sizes
                .push(last_pos + bytes_read as u64);
            // Remove trailing whitespace
            line = line.trim_end().to_string();
            // Store the line (using mem::take to avoid clone)
            self.lines.push(std::mem::take(&mut line));
        }
        Ok(true)
    }

    fn wait_for_enter_key(&self) -> UResult<()> {
        if !self.stdout.is_tty() {
            return Ok(());
        }
        loop {
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press,
                    ..
                }) = event::read()?
                {
                    return Ok(());
                }
            }
        }
    }

    fn handle_pattern_search(&mut self) -> UResult<()> {
        if self.pattern.is_none() {
            return Ok(());
        }
        match self.search_pattern_in_file() {
            Some(line) => self.upper_mark = line,
            None => {
                self.pattern = None;
                write!(
                    self.stdout,
                    "\r{}{} ({}){}",
                    Attribute::Reverse,
                    translate!("more-error-pattern-not-found"),
                    translate!("more-press-return"),
                    Attribute::Reset,
                )?;
                self.stdout.flush()?;
                self.wait_for_enter_key()?;
            }
        }
        Ok(())
    }

    fn search_pattern_in_file(&mut self) -> Option<usize> {
        let pattern = self.pattern.clone().expect("pattern should be set");
        let mut line_num = self.upper_mark;
        loop {
            match self.get_line(line_num) {
                Some(line) if line.contains(&pattern) => return Some(line_num),
                Some(_) => line_num += 1,
                None => return None,
            }
        }
    }

    fn get_line(&mut self, index: usize) -> Option<&String> {
        match self.read_until_line(index) {
            Ok(true) => self.lines.get(index),
            _ => None,
        }
    }

    fn display_multi_file_header(&mut self) -> UResult<()> {
        self.stdout.queue(Clear(ClearType::CurrentLine))?;
        self.stdout.write_all(
            MULTI_FILE_TOP_PROMPT
                .replace("{}", self.file_name.unwrap_or_default())
                .as_bytes(),
        )?;
        self.content_rows = self
            .content_rows
            .saturating_sub(MULTI_FILE_TOP_PROMPT.lines().count());
        Ok(())
    }

    fn reset_multi_file_header(&mut self) {
        self.content_rows = self
            .content_rows
            .saturating_add(MULTI_FILE_TOP_PROMPT.lines().count());
    }

    fn update_display(&mut self, options: &Options) -> UResult<()> {
        if options.print_over {
            self.stdout
                .execute(MoveTo(0, 0))?
                .execute(Clear(ClearType::FromCursorDown))?;
        } else if options.clean_print {
            self.stdout
                .execute(Clear(ClearType::All))?
                .execute(MoveTo(0, 0))?;
        }
        Ok(())
    }

    /// Process user input events until exit
    fn process_events(&mut self, options: &Options) -> UResult<()> {
        loop {
            if !event::poll(Duration::from_millis(100))? {
                continue;
            }
            let mut wrong_key = None;
            match event::read()? {
                // --- Quit commands ---
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
                ) => {
                    reset_term()?;
                    std::process::exit(0);
                }

                // --- Forward Navigation ---
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::PageDown | KeyCode::Char(' '),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    if self.eof_reached {
                        return Ok(());
                    }
                    self.page_down();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('j'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    if self.eof_reached {
                        return Ok(());
                    }
                    self.next_line();
                }

                // --- Backward Navigation ---
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::PageUp,
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    self.page_up();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('k'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    self.prev_line();
                }

                // --- Terminal events ---
                Event::Resize(col, row) => {
                    self.page_resize(col, row, options.lines);
                }

                // --- Skip key release events ---
                Event::Key(KeyEvent {
                    kind: KeyEventKind::Release,
                    ..
                }) => continue,

                // --- Handle unknown keys ---
                Event::Key(KeyEvent {
                    code: KeyCode::Char(k),
                    ..
                }) => wrong_key = Some(k),

                // --- Ignore other events ---
                _ => continue,
            }
            self.update_display(options)?;
            self.draw(wrong_key)?;
        }
    }

    fn page_down(&mut self) {
        // Move the viewing window down by the number of lines to display
        self.upper_mark = self.upper_mark.saturating_add(self.content_rows);
    }

    fn next_line(&mut self) {
        // Move the viewing window down by one line
        self.upper_mark = self.upper_mark.saturating_add(1);
    }

    fn page_up(&mut self) {
        self.eof_reached = false;
        // Move the viewing window up by the number of lines to display
        self.upper_mark = self
            .upper_mark
            .saturating_sub(self.content_rows.saturating_add(self.lines_squeezed));
        if self.squeeze {
            // Move upper mark to the first non-empty line
            while self.upper_mark > 0 {
                let line = self.lines.get(self.upper_mark).expect("line should exist");
                if !line.trim().is_empty() {
                    break;
                }
                self.upper_mark = self.upper_mark.saturating_sub(1);
            }
        }
    }

    fn prev_line(&mut self) {
        self.eof_reached = false;
        // Move the viewing window up by one line
        self.upper_mark = self.upper_mark.saturating_sub(1);
    }

    // TODO: Deal with column size changes.
    fn page_resize(&mut self, _col: u16, row: u16, option_line: Option<u16>) {
        if option_line.is_none() {
            self.content_rows = row.saturating_sub(1) as usize;
        }
    }

    fn draw(&mut self, wrong_key: Option<char>) -> UResult<()> {
        self.draw_lines()?;
        self.draw_status_bar(wrong_key);
        self.stdout.flush()?;
        Ok(())
    }

    fn draw_lines(&mut self) -> UResult<()> {
        // Clear current prompt line
        self.stdout.queue(Clear(ClearType::CurrentLine))?;
        // Reset squeezed lines counter
        self.lines_squeezed = 0;
        // Display lines until we've filled the screen
        let mut lines_printed = 0;
        let mut index = self.upper_mark;
        while lines_printed < self.content_rows {
            // Load the required line or stop at EOF
            if !self.read_until_line(index)? {
                self.eof_reached = true;
                self.upper_mark = index.saturating_sub(self.content_rows);
                break;
            }
            // Skip line if it should be squeezed
            if self.should_squeeze_line(index) {
                self.lines_squeezed += 1;
                index += 1;
                continue;
            }
            // Display the line
            let mut line = self.lines[index].clone();
            if let Some(pattern) = &self.pattern {
                // Highlight the pattern in the line
                line = line.replace(
                    pattern,
                    &format!("{}{pattern}{}", Attribute::Reverse, Attribute::Reset),
                );
            }
            self.stdout.write_all(format!("\r{line}\n").as_bytes())?;
            lines_printed += 1;
            index += 1;
        }
        // Fill remaining lines with `~`
        while lines_printed < self.content_rows {
            self.stdout.write_all(b"\r~\n")?;
            lines_printed += 1;
        }
        Ok(())
    }

    fn should_squeeze_line(&self, index: usize) -> bool {
        // Only squeeze if enabled and not the first line
        if !self.squeeze || index == 0 {
            return false;
        }
        // Squeeze only if both current and previous lines are empty
        match (self.lines.get(index), self.lines.get(index - 1)) {
            (Some(current), Some(previous)) => current.is_empty() && previous.is_empty(),
            _ => false,
        }
    }

    fn draw_status_bar(&mut self, wrong_key: Option<char>) {
        // Calculate the index of the last visible line
        let lower_mark =
            (self.upper_mark + self.content_rows).min(self.lines.len().saturating_sub(1));
        // Determine progress information to display
        // - Show next file name when at EOF and there is a next file
        // - Otherwise show percentage of the file read (if available)
        let progress_info = if self.eof_reached && self.next_file.is_some() {
            format!(" (Next file: {})", self.next_file.unwrap())
        } else if let Some(file_size) = self.file_size {
            // For files, show percentage or END
            let position = self
                .cumulative_line_sizes
                .get(lower_mark)
                .copied()
                .unwrap_or_default();
            if file_size == 0 {
                " (END)".to_string()
            } else {
                let percentage = (position as f64 / file_size as f64 * 100.0).round() as u16;
                if percentage >= 100 {
                    " (END)".to_string()
                } else {
                    format!(" ({percentage}%)")
                }
            }
        } else {
            // For stdin, don't show percentage
            String::new()
        };
        // Base status message with progress info
        let file_name = self.file_name.unwrap_or(":");
        let status = format!("{file_name}{progress_info}");
        // Add appropriate user feedback based on silent mode and key input:
        // - In silent mode: show help text or unknown key message
        // - In normal mode: ring bell (BELL char) on wrong key or show basic prompt
        let banner = match (self.silent, wrong_key) {
            (true, Some(key)) => format!(
                "{status}[{}]",
                translate!(
                    "more-error-unknown-key",
                    "key" => key,
                )
            ),
            (true, None) => format!("{status}{}", translate!("more-help-message")),
            (false, Some(_)) => format!("{status}{BELL}"),
            (false, None) => status,
        };
        // Draw the status bar at the bottom of the screen
        write!(
            self.stdout,
            "\r{}{banner}{}",
            Attribute::Reverse,
            Attribute::Reset
        )
        .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::Seek,
        ops::{Deref, DerefMut},
    };

    use super::*;
    use tempfile::tempfile;

    impl Deref for OutputType {
        type Target = Vec<u8>;
        fn deref(&self) -> &Vec<u8> {
            match self {
                Self::Test(buf) => buf,
                _ => unreachable!(),
            }
        }
    }

    impl DerefMut for OutputType {
        fn deref_mut(&mut self) -> &mut Vec<u8> {
            match self {
                Self::Test(buf) => buf,
                _ => unreachable!(),
            }
        }
    }

    struct TestPagerBuilder {
        content: String,
        options: Options,
        rows: u16,
        next_file: Option<&'static str>,
    }

    impl Default for TestPagerBuilder {
        fn default() -> Self {
            Self {
                content: String::new(),
                options: Options {
                    silent: false,
                    _logical: false,
                    _exit_on_eof: false,
                    _no_pause: false,
                    print_over: false,
                    clean_print: false,
                    squeeze: false,
                    lines: None,
                    from_line: 0,
                    pattern: None,
                },
                rows: 10,
                next_file: None,
            }
        }
    }

    #[allow(dead_code)]
    impl TestPagerBuilder {
        fn new(content: &str) -> Self {
            Self {
                content: content.to_string(),
                ..Default::default()
            }
        }

        fn build(mut self) -> Pager<'static> {
            let mut tmpfile = tempfile().unwrap();
            tmpfile.write_all(self.content.as_bytes()).unwrap();
            tmpfile.rewind().unwrap();
            let out = OutputType::Test(Vec::new());
            if let Some(rows) = self.options.lines {
                self.rows = rows;
            }
            Pager::new(
                InputType::File(BufReader::new(tmpfile)),
                self.rows,
                None,
                self.next_file,
                &self.options,
                out,
            )
            .unwrap()
        }

        fn silent(mut self) -> Self {
            self.options.silent = true;
            self
        }

        fn print_over(mut self) -> Self {
            self.options.print_over = true;
            self
        }

        fn clean_print(mut self) -> Self {
            self.options.clean_print = true;
            self
        }

        fn squeeze(mut self) -> Self {
            self.options.squeeze = true;
            self
        }

        fn lines(mut self, lines: u16) -> Self {
            self.options.lines = Some(lines);
            self
        }

        #[allow(clippy::wrong_self_convention)]
        fn from_line(mut self, from_line: usize) -> Self {
            self.options.from_line = from_line;
            self
        }

        fn pattern(mut self, pattern: &str) -> Self {
            self.options.pattern = Some(pattern.to_owned());
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

    #[test]
    fn test_get_line_and_len() {
        let content = "a\n\tb\nc\n";
        let mut pager = TestPagerBuilder::new(content).build();
        assert_eq!(pager.get_line(1).unwrap(), "\tb");
        assert_eq!(pager.cumulative_line_sizes.len(), 2);
        assert_eq!(pager.cumulative_line_sizes[1], 5);
    }

    #[test]
    fn test_navigate_page() {
        // create 10 lines "0\n".."9\n"
        let content = (0..10).map(|i| i.to_string() + "\n").collect::<String>();

        // content_rows = rows - 1 = 10 - 1 = 9
        let mut pager = TestPagerBuilder::new(&content).build();
        assert_eq!(pager.upper_mark, 0);

        pager.page_down();
        assert_eq!(pager.upper_mark, pager.content_rows);
        pager.draw(None).unwrap();
        let mut stdout = String::from_utf8_lossy(&pager.stdout);
        assert!(stdout.contains("9\n"));
        assert!(!stdout.contains("8\n"));
        assert_eq!(pager.upper_mark, 1); // EOF reached: upper_mark = 10 - content_rows = 1

        pager.page_up();
        assert_eq!(pager.upper_mark, 0);

        pager.next_line();
        assert_eq!(pager.upper_mark, 1);

        pager.prev_line();
        assert_eq!(pager.upper_mark, 0);
        pager.stdout.clear();
        pager.draw(None).unwrap();
        stdout = String::from_utf8_lossy(&pager.stdout);
        assert!(stdout.contains("0\n"));
        assert!(!stdout.contains("9\n")); // only lines 0 to 8 should be displayed
    }

    #[test]
    fn test_silent_mode() {
        let content = (0..5).map(|i| i.to_string() + "\n").collect::<String>();
        let mut pager = TestPagerBuilder::new(&content)
            .from_line(3)
            .silent()
            .build();
        pager.draw_status_bar(None);
        let stdout = String::from_utf8_lossy(&pager.stdout);
        assert!(stdout.contains(&translate!("more-help-message")));
    }

    #[test]
    fn test_squeeze() {
        let content = "Line 0\n\n\n\nLine 4\n\n\nLine 7\n";
        let mut pager = TestPagerBuilder::new(content).lines(6).squeeze().build();
        assert_eq!(pager.content_rows, 5); // 1 line for the status bar

        // load all lines
        assert!(pager.read_until_line(7).unwrap());
        //  back‑to‑back empty lines → should squeeze
        assert!(pager.should_squeeze_line(2));
        assert!(pager.should_squeeze_line(3));
        assert!(pager.should_squeeze_line(6));
        // non‑blank or first line should not be squeezed
        assert!(!pager.should_squeeze_line(0));
        assert!(!pager.should_squeeze_line(1));
        assert!(!pager.should_squeeze_line(4));
        assert!(!pager.should_squeeze_line(5));
        assert!(!pager.should_squeeze_line(7));

        pager.draw(None).unwrap();
        let stdout = String::from_utf8_lossy(&pager.stdout);
        assert!(stdout.contains("Line 0"));
        assert!(stdout.contains("Line 4"));
        assert!(stdout.contains("Line 7"));
    }

    #[test]
    fn test_lines_option() {
        let content = (0..5).map(|i| i.to_string() + "\n").collect::<String>();

        // Output zero lines succeeds
        let mut pager = TestPagerBuilder::new(&content).lines(0).build();
        pager.draw(None).unwrap();
        let mut stdout = String::from_utf8_lossy(&pager.stdout);
        assert!(!stdout.is_empty());

        // Output two lines
        let mut pager = TestPagerBuilder::new(&content).lines(3).build();
        assert_eq!(pager.content_rows, 3 - 1); // 1 line for the status bar
        pager.draw(None).unwrap();
        stdout = String::from_utf8_lossy(&pager.stdout);
        assert!(stdout.contains("0\n"));
        assert!(stdout.contains("1\n"));
        assert!(!stdout.contains("2\n"));
    }

    #[test]
    fn test_from_line_option() {
        let content = (0..5).map(|i| i.to_string() + "\n").collect::<String>();

        // Output from first line
        let mut pager = TestPagerBuilder::new(&content).from_line(0).build();
        assert!(pager.handle_from_line().is_ok());
        pager.draw(None).unwrap();
        let stdout = String::from_utf8_lossy(&pager.stdout);
        assert!(stdout.contains("0\n"));

        // Output from second line
        pager = TestPagerBuilder::new(&content).from_line(1).build();
        assert!(pager.handle_from_line().is_ok());
        pager.draw(None).unwrap();
        let stdout = String::from_utf8_lossy(&pager.stdout);
        assert!(stdout.contains("1\n"));
        assert!(!stdout.contains("0\n"));

        // Output from out of range line
        pager = TestPagerBuilder::new(&content).from_line(99).build();
        assert!(pager.handle_from_line().is_ok());
        assert_eq!(pager.upper_mark, 0);
        let stdout = String::from_utf8_lossy(&pager.stdout);
        assert!(stdout.contains(&translate!(
            "more-error-cannot-seek-to-line",
            "line" => "100"
        )));
    }

    #[test]
    fn test_search_pattern_found() {
        let content = "foo\nbar\nbaz\n";
        let mut pager = TestPagerBuilder::new(content).pattern("bar").build();
        assert!(pager.handle_pattern_search().is_ok());
        assert_eq!(pager.upper_mark, 1);
        pager.draw(None).unwrap();
        let stdout = String::from_utf8_lossy(&pager.stdout);
        assert!(stdout.contains("bar"));
        assert!(!stdout.contains("foo"));
    }

    #[test]
    fn test_search_pattern_not_found() {
        let content = "foo\nbar\nbaz\n";
        let mut pager = TestPagerBuilder::new(content).pattern("qux").build();
        assert!(pager.handle_pattern_search().is_ok());
        let stdout = String::from_utf8_lossy(&pager.stdout);
        assert!(stdout.contains(&translate!("more-error-pattern-not-found")));
        assert_eq!(pager.pattern, None);
        assert_eq!(pager.upper_mark, 0);
    }

    #[test]
    fn test_wrong_key() {
        let mut pager = TestPagerBuilder::default().silent().build();
        pager.draw_status_bar(Some('x'));
        let stdout = String::from_utf8_lossy(&pager.stdout);
        assert!(stdout.contains(&translate!(
            "more-error-unknown-key",
            "key" => "x"
        )));

        pager = TestPagerBuilder::default().build();
        pager.draw_status_bar(Some('x'));
        let stdout = String::from_utf8_lossy(&pager.stdout);
        assert!(stdout.contains(&BELL.to_string()));
    }
}
