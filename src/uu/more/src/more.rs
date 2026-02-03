// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore keymods

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

/// The prompt to be displayed at the top of the screen when viewing multiple files, with the file name in the middle
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

struct Options<'args> {
    silent: bool,
    _logical: bool, // not implemented
    exit_on_eof: bool,
    _no_pause: bool, // not implemented
    print_over: bool,
    clean_print: bool,
    squeeze: bool,
    lines: Option<u16>,
    from_line: usize,
    pattern: Option<&'args str>,
}

impl<'args> Options<'args> {
    fn from(matches: &'args ArgMatches) -> Self {
        // If lines is None, use terminal height
        let lines = matches
            .get_one::<u16>(options::LINES)
            .or_else(|| matches.get_one::<u16>(options::NUMBER))
            .filter(|&&n| n > 0)
            // add 1 to the number of lines to display since the last line is used for the banner
            .map(|n| n + 1);

        let from_line = matches
            .get_one::<usize>(options::FROM_LINE)
            // convert from 1-indexed to 0-indexed
            .map_or(0, |n| n.saturating_sub(1));

        // exit_on_eof is enabled by default if:
        // - POSIXLY_CORRECT environment variable is not set, or
        // - not executed on terminal
        let posixly_correct = std::env::var_os("POSIXLY_CORRECT").is_some();
        let is_tty = stdout().is_tty();
        let explicit_exit_on_eof = matches.get_flag(options::EXIT_ON_EOF);
        let exit_on_eof = !posixly_correct || !is_tty || explicit_exit_on_eof;

        let pattern = matches
            .get_one::<String>(options::PATTERN)
            .map(|s| s.as_str());

        Self {
            silent: matches.get_flag(options::SILENT),
            _logical: matches.get_flag(options::LOGICAL),
            exit_on_eof,
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
        while let (Some(file_os), next_file_os) = (files_iter.next(), files_iter.peek()) {
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
            more(
                Input::from_file(opened_file)?,
                length > 1,
                Some(&file.to_string_lossy()),
                next_file_os.map(|f| f.to_string_lossy()).as_deref(),
                &mut options,
            )?;
        }
    } else {
        let stdin = stdin();
        if stdin.is_tty() {
            // stdin is not a pipe
            return Err(UUsageError::new(1, MoreError::BadUsage.to_string()));
        }
        more(Input::from_stdin(stdin), false, None, None, &mut options)?;
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

struct Input {
    reader: Box<dyn BufRead>,
    file_size: Option<u64>,
}

impl Input {
    fn from_file(file: File) -> std::io::Result<Self> {
        let file_size = Some(file.metadata()?.len());
        Ok(Self {
            reader: Box::new(BufReader::new(file)),
            file_size,
        })
    }

    fn from_stdin(stdin: Stdin) -> Self {
        Self {
            reader: Box::new(stdin.lock()),
            file_size: None,
        }
    }

    fn read_line(&mut self, buf: &mut String) -> std::io::Result<usize> {
        self.reader.read_line(buf)
    }

    fn len(&self) -> Option<u64> {
        self.file_size
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
    input: Input,
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
    let pattern = options.pattern;
    let mut pager = Pager::new(input, out, rows, file_name, next_file, pattern, options)?;
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
    // Exit immediately if exit_on_eof is enabled and EOF is reached
    if pager.exit_on_eof && pager.eof_reached {
        return Ok(());
    }
    // Reset multi-file settings after initial display
    if multiple_file {
        pager.reset_multi_file_header();
        options.from_line = 0;
    }
    // Main event loop
    pager.process_events(options)
}

struct Pager<'args> {
    input: Input,
    stdout: OutputType,
    /// Total size of the file in bytes (only available for file inputs)
    file_size: Option<u64>,
    file_name: Option<&'args str>,
    next_file_name: Option<&'args str>,
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
    eof_reached: bool,
    pattern: Option<&'args str>,
    /// Do not ring bell on unknown key press
    silent: bool,
    /// Squeeze blank lines into a single line
    squeeze: bool,
    /// Exit immediately when EOF is reached
    exit_on_eof: bool,
}

impl<'args> Pager<'args> {
    fn new(
        input: Input,
        stdout: OutputType,
        rows: u16,
        file_name: Option<&'args str>,
        next_file_name: Option<&'args str>,
        pattern: Option<&'args str>,
        options: &Options<'args>,
    ) -> UResult<Self> {
        // Reserve one line for the status bar, ensuring at least one content row
        let content_rows = rows.saturating_sub(1).max(1) as usize;
        let file_size = input.len();
        let pager = Self {
            input,
            stdout,
            file_size,
            file_name,
            next_file_name,
            lines: Vec::with_capacity(content_rows),
            cumulative_line_sizes: Vec::new(),
            upper_mark: options.from_line,
            content_rows,
            lines_squeezed: 0,
            eof_reached: false,
            pattern,
            silent: options.silent,
            squeeze: options.squeeze,
            exit_on_eof: options.exit_on_eof,
        };
        Ok(pager)
    }

    fn clear_line(&mut self) -> std::io::Result<()> {
        if self.stdout.is_tty() {
            self.stdout.queue(Clear(ClearType::CurrentLine))?;
        }
        Ok(())
    }

    fn highlight_text(&self, text: &str) -> String {
        if self.stdout.is_tty() {
            format!("{}{text}{}", Attribute::Reverse, Attribute::Reset)
        } else {
            text.to_string()
        }
    }

    fn handle_from_line(&mut self) -> UResult<()> {
        if !self.read_until_line(self.upper_mark)? {
            let msg = format!(
                "{} ({})",
                translate!(
                    "more-error-cannot-seek-to-line",
                    "line" => (self.upper_mark + 1)
                ),
                translate!("more-press-return"),
            );
            write!(self.stdout, "\r{}", self.highlight_text(&msg))?;
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
                let msg = format!(
                    "{} ({})",
                    translate!("more-error-pattern-not-found"),
                    translate!("more-press-return"),
                );
                write!(self.stdout, "\r{}", self.highlight_text(&msg))?;
                self.stdout.flush()?;
                self.wait_for_enter_key()?;
            }
        }
        Ok(())
    }

    fn search_pattern_in_file(&mut self) -> Option<usize> {
        let pattern = self.pattern.expect("pattern should be set");
        let mut line_num = self.upper_mark;
        loop {
            match self.get_line(line_num) {
                Some(line) if line.contains(pattern) => return Some(line_num),
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
        self.clear_line()?;
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
        if self.stdout.is_tty() {
            if options.print_over {
                // Clear the whole screen and then display the text
                self.stdout
                    .execute(MoveTo(0, 0))?
                    .execute(Clear(ClearType::FromCursorDown))?;
            } else if options.clean_print {
                // Paint each screen from the top,
                // clearing the remainder of each line as it is displayed
                self.stdout
                    .execute(Clear(ClearType::All))?
                    .execute(MoveTo(0, 0))?;
            }
        }
        Ok(())
    }

    /// Process user input events until exit
    fn process_events(&mut self, options: &Options) -> UResult<()> {
        loop {
            if !event::poll(Duration::from_millis(100))? {
                continue;
            }
            let mut unknown_key = None;
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
                    if self.eof_reached && self.exit_on_eof {
                        return Ok(());
                    }
                    self.page_down();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('j'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    if self.eof_reached && self.exit_on_eof {
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
                    self.resize_page(col, row, options.lines);
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
                }) => unknown_key = Some(k),

                // --- Ignore other events ---
                _ => continue,
            }
            self.update_display(options)?;
            self.draw(unknown_key)?;
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
    fn resize_page(&mut self, _col: u16, row: u16, option_line: Option<u16>) {
        if option_line.is_none() {
            self.content_rows = row.saturating_sub(1) as usize;
        }
    }

    fn draw(&mut self, unknown_key: Option<char>) -> UResult<()> {
        self.draw_lines()?;
        self.draw_status_bar(unknown_key)?;
        self.stdout.flush()?;
        Ok(())
    }

    fn draw_lines(&mut self) -> UResult<()> {
        // Clear current prompt line
        self.clear_line()?;
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
                let highlighted = self.highlight_text(pattern);
                line = line.replace(pattern, &highlighted);
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

    fn draw_status_bar(&mut self, unknown_key: Option<char>) -> UResult<()> {
        // Calculate the index of the last visible line
        let lower_mark =
            (self.upper_mark + self.content_rows).min(self.lines.len().saturating_sub(1));
        // Determine progress information to display
        // - Show next file name when at EOF and there is a next file
        // - Otherwise show percentage of the file read (if available)
        let progress_info = if self.eof_reached {
            self.next_file_name
                .as_ref()
                .map(|next_file| format!(" (Next file: {next_file})"))
                .unwrap_or_default()
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
        let banner = match (self.silent, unknown_key) {
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
        let styled_banner = self.highlight_text(&banner);
        write!(self.stdout, "\r{styled_banner}")?;
        Ok(())
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
        options: Options<'static>,
        rows: u16,
        next_file_name: Option<&'static str>,
    }

    impl Default for TestPagerBuilder {
        fn default() -> Self {
            Self {
                content: String::new(),
                options: Options {
                    silent: false,
                    _logical: false,
                    exit_on_eof: false,
                    _no_pause: false,
                    print_over: false,
                    clean_print: false,
                    squeeze: false,
                    lines: None,
                    from_line: 0,
                    pattern: None,
                },
                rows: 24,
                next_file_name: None,
            }
        }
    }

    macro_rules! option_builder {
        ($name:ident: bool) => {
            #[allow(clippy::used_underscore_binding)]
            fn $name(mut self) -> Self {
                self.options.$name = true;
                self
            }
        };
        ($name:ident: Option<$inner:ty>) => {
            fn $name(mut self, value: $inner) -> Self {
                self.options.$name = Some(value);
                self
            }
        };
        ($name:ident: $type:ty) => {
            #[allow(clippy::wrong_self_convention)]
            fn $name(mut self, value: $type) -> Self {
                self.options.$name = value;
                self
            }
        };
        ($name:ident => $field:ident: $type:ty) => {
            fn $name(mut self, value: $type) -> Self {
                self.$field = value;
                self
            }
        };
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
                Input::from_file(tmpfile).unwrap(),
                out,
                self.rows,
                None,
                self.next_file_name,
                self.options.pattern,
                &self.options,
            )
            .unwrap()
        }

        option_builder!(silent: bool);
        option_builder!(_logical: bool);
        option_builder!(exit_on_eof: bool);
        option_builder!(_no_pause: bool);
        option_builder!(print_over: bool);
        option_builder!(clean_print: bool);
        option_builder!(squeeze: bool);
        option_builder!(lines: Option<u16>);
        option_builder!(from_line: usize);
        option_builder!(pattern: Option<&'static str>);
        option_builder!(rows => rows: u16);
        option_builder!(next_file_name => next_file_name: Option<&'static str>);
    }

    fn get_content_lines(stdout: &[u8]) -> Vec<String> {
        let stdout = String::from_utf8_lossy(stdout);
        stdout
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| line != "~" && !line.starts_with(':'))
            .collect()
    }

    fn get_status_bar(stdout: &[u8]) -> String {
        let stdout = String::from_utf8_lossy(stdout);
        stdout
            .lines()
            .last()
            .map(|line| line.trim().to_string())
            .unwrap_or_default()
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
        let lines = (0..24).map(|i| i.to_string()).collect::<Vec<String>>();
        let mut pager = TestPagerBuilder::new(&lines.join("\n")).build();
        assert_eq!(pager.upper_mark, 0);

        pager.page_down();
        assert_eq!(pager.upper_mark, pager.content_rows);

        pager.draw(None).unwrap();
        let content_lines = get_content_lines(&pager.stdout);
        assert_eq!(
            content_lines,
            lines[pager.content_rows..],
            "Pager should display the next content_rows lines"
        );
        assert_eq!(pager.upper_mark, 1);

        pager.page_up();
        assert_eq!(pager.upper_mark, 0);

        pager.next_line();
        assert_eq!(pager.upper_mark, 1);

        pager.prev_line();
        assert_eq!(pager.upper_mark, 0);
        pager.stdout.clear();

        pager.draw(None).unwrap();
        let content_lines = get_content_lines(&pager.stdout);
        assert_eq!(
            content_lines,
            lines[0..pager.content_rows],
            "Pager should display the first content_rows lines"
        );
    }

    #[test]
    fn test_no_next_file_in_status_bar() {
        let content = "line1\nline2\n";
        let mut pager = TestPagerBuilder::new(content).build();
        pager.draw(None).unwrap();
        let status_bar = get_status_bar(&pager.stdout);
        assert_eq!(status_bar, ":", "Status bar should be empty");
    }

    #[test]
    fn test_next_file_in_status_bar() {
        let content = "line1\nline2\n";
        let next_file_name = "next.txt";
        let mut pager = TestPagerBuilder::new(content)
            .next_file_name(Some(next_file_name))
            .build();
        pager.draw(None).unwrap();
        let status_bar = get_status_bar(&pager.stdout);
        assert_eq!(
            status_bar,
            format!(": (Next file: {next_file_name})"),
            "Status bar should show the next file name"
        );
    }

    #[test]
    fn test_silent_mode() {
        let content = (0..5).map(|i| i.to_string()).collect::<String>();
        let mut pager = TestPagerBuilder::new(&content).silent().build();
        pager.draw_status_bar(None).unwrap();
        let status_bar = get_status_bar(&pager.stdout);
        assert!(
            status_bar.contains(&translate!("more-help-message")),
            "Status bar should show the help message"
        );
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
        let content_lines = get_content_lines(&pager.stdout);
        assert_eq!(content_lines[0], "Line 0");
        assert_eq!(content_lines[2], "Line 4");
        assert_eq!(content_lines[4], "Line 7");
    }

    #[test]
    fn test_lines_option() {
        let lines = (0..5).map(|i| i.to_string()).collect::<Vec<String>>();

        let mut pager = TestPagerBuilder::new(&lines.join("\n")).lines(0).build();
        pager.draw(None).unwrap();
        let content_lines = get_content_lines(&pager.stdout);
        assert!(
            !content_lines.is_empty(),
            "lines = 0 should use default rows"
        );

        let lines_opt = 3;
        let mut pager = TestPagerBuilder::new(&(lines.join("\n")))
            .lines(lines_opt as u16)
            .build();
        let output_lines = lines_opt - 1; // 1 line for the status bar
        assert_eq!(pager.content_rows, output_lines);
        pager.draw(None).unwrap();
        let content_lines = get_content_lines(&pager.stdout);
        assert_eq!(content_lines, lines[0..output_lines]);
    }

    #[test]
    fn test_from_line_option() {
        let lines = vec!["0", "1", "2", "3", "4"];
        let content = lines.join("\n");

        // Output from first line
        let mut pager = TestPagerBuilder::new(&content).from_line(0).build();
        assert!(pager.handle_from_line().is_ok());
        pager.draw(None).unwrap();
        let content_lines = get_content_lines(&pager.stdout);
        assert_eq!(content_lines, lines);

        // Output from second line
        pager = TestPagerBuilder::new(&content).from_line(1).build();
        assert!(pager.handle_from_line().is_ok());
        pager.draw(None).unwrap();
        let content_lines = get_content_lines(&pager.stdout);
        assert_eq!(content_lines, lines[1..]);

        // Out of range error
        pager = TestPagerBuilder::new(&content).from_line(99).build();
        assert!(pager.handle_from_line().is_ok());
        assert_eq!(pager.upper_mark, 0);
        let status_bar = get_status_bar(&pager.stdout);
        assert!(status_bar.contains(&translate!(
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
        let content_lines = get_content_lines(&pager.stdout);
        assert_eq!(content_lines, vec!["bar", "baz"]);
    }

    #[test]
    fn test_search_pattern_not_found() {
        let content = "foo\nbar\nbaz\n";
        let mut pager = TestPagerBuilder::new(content).pattern("qux").build();
        assert!(pager.handle_pattern_search().is_ok());
        assert_eq!(pager.pattern, None);
        assert_eq!(pager.upper_mark, 0);
        let status_bar = get_status_bar(&pager.stdout);
        assert!(status_bar.contains(&translate!("more-error-pattern-not-found")));
    }

    #[test]
    fn test_unknown_key() {
        let mut silent_pager = TestPagerBuilder::default().silent().build();
        silent_pager.draw_status_bar(Some('x')).unwrap();
        let status_bar = get_status_bar(&silent_pager.stdout);
        assert!(status_bar.contains(&translate!(
            "more-error-unknown-key",
            "key" => "x"
        )));

        let mut pager = TestPagerBuilder::default().build();
        pager.draw_status_bar(Some('x')).unwrap();
        let status_bar = get_status_bar(&pager.stdout);
        assert!(status_bar.contains(&BELL.to_string()));
    }
}
