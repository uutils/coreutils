//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Akira Hayakawa <ruby.wktk@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) PREFIXaa

#[macro_use]
extern crate uucore;

use std::char;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Read, Result, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

static NAME: &str = "split";
static VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();

    let mut opts = getopts::Options::new();

    opts.optopt(
        "a",
        "suffix-length",
        "use suffixes of length N (default 2)",
        "N",
    );
    opts.optopt("b", "bytes", "put SIZE bytes per output file", "SIZE");
    opts.optopt(
        "C",
        "line-bytes",
        "put at most SIZE bytes of lines per output file",
        "SIZE",
    );
    opts.optflag(
        "d",
        "numeric-suffixes",
        "use numeric suffixes instead of alphabetic",
    );
    opts.optopt(
        "",
        "additional-suffix",
        "additional suffix to append to output file names",
        "SUFFIX",
    );
    opts.optopt(
        "",
        "filter",
        "write to shell COMMAND file name is $FILE",
        "COMMAND",
    );
    opts.optopt("l", "lines", "put NUMBER lines per output file", "NUMBER");
    opts.optflag(
        "",
        "verbose",
        "print a diagnostic just before each output file is opened",
    );
    opts.optflag("h", "help", "display help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f),
    };

    if matches.opt_present("h") {
        let msg = format!(
            "{0} {1}

Usage:
  {0} [OPTION]... [INPUT [PREFIX]]

Output fixed-size pieces of INPUT to PREFIXaa, PREFIX ab, ...; default
size is 1000, and default PREFIX is 'x'. With no INPUT, or when INPUT is
-, read standard input.",
            NAME, VERSION
        );

        println!(
            "{}\nSIZE may have a multiplier suffix: b for 512, k for 1K, m for 1 Meg.",
            opts.usage(&msg)
        );
        return 0;
    }

    if matches.opt_present("V") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    let mut settings = Settings {
        prefix: "".to_owned(),
        numeric_suffix: false,
        suffix_length: 0,
        additional_suffix: "".to_owned(),
        input: "".to_owned(),
        filter: None,
        strategy: "".to_owned(),
        strategy_param: "".to_owned(),
        verbose: false,
    };

    settings.numeric_suffix = matches.opt_present("d");

    settings.suffix_length = match matches.opt_str("a") {
        Some(n) => match n.parse() {
            Ok(m) => m,
            Err(e) => crash!(1, "cannot parse num: {}", e),
        },
        None => 2,
    };

    settings.additional_suffix = if matches.opt_present("additional-suffix") {
        matches.opt_str("additional-suffix").unwrap()
    } else {
        "".to_owned()
    };

    settings.verbose = matches.opt_present("verbose");

    settings.strategy = "l".to_owned();
    settings.strategy_param = "1000".to_owned();
    let strategies = vec!["b", "C", "l"];
    for e in &strategies {
        if let Some(a) = matches.opt_str(*e) {
            if settings.strategy == "l" {
                settings.strategy = (*e).to_owned();
                settings.strategy_param = a;
            } else {
                crash!(1, "{}: cannot split in more than one way", NAME)
            }
        }
    }

    let mut v = matches.free.iter();
    let (input, prefix) = match (v.next(), v.next()) {
        (Some(a), None) => (a.to_owned(), "x".to_owned()),
        (Some(a), Some(b)) => (a.clone(), b.clone()),
        (None, _) => ("-".to_owned(), "x".to_owned()),
    };
    settings.input = input;
    settings.prefix = prefix;

    settings.filter = if let Some(filter) = matches.opt_str("filter") {
        Some(filter)
    } else {
        None
    };

    split(&settings)
}

struct Settings {
    prefix: String,
    numeric_suffix: bool,
    suffix_length: usize,
    additional_suffix: String,
    input: String,
    /// When supplied, a shell command to output to instead of xaa, xab …
    filter: Option<String>,
    strategy: String,
    strategy_param: String,
    verbose: bool,
}

struct SplitControl {
    current_line: String,   // Don't touch
    request_new_file: bool, // Splitter implementation requests new file
}

trait Splitter {
    // Consume the current_line and return the consumed string
    fn consume(&mut self, _: &mut SplitControl) -> String;
}

struct LineSplitter {
    saved_lines_to_write: usize,
    lines_to_write: usize,
}

impl LineSplitter {
    fn new(settings: &Settings) -> LineSplitter {
        let n = match settings.strategy_param.parse() {
            Ok(a) => a,
            Err(e) => crash!(1, "invalid number of lines: {}", e),
        };
        LineSplitter {
            saved_lines_to_write: n,
            lines_to_write: n,
        }
    }
}

impl Splitter for LineSplitter {
    fn consume(&mut self, control: &mut SplitControl) -> String {
        self.lines_to_write -= 1;
        if self.lines_to_write == 0 {
            self.lines_to_write = self.saved_lines_to_write;
            control.request_new_file = true;
        }
        control.current_line.clone()
    }
}

struct ByteSplitter {
    saved_bytes_to_write: usize,
    bytes_to_write: usize,
    break_on_line_end: bool,
    require_whole_line: bool,
}

impl ByteSplitter {
    fn new(settings: &Settings) -> ByteSplitter {
        let mut strategy_param: Vec<char> = settings.strategy_param.chars().collect();
        let suffix = strategy_param.pop().unwrap();
        let multiplier = match suffix {
            '0'..='9' => 1usize,
            'b' => 512usize,
            'k' => 1024usize,
            'm' => 1024usize * 1024usize,
            _ => crash!(1, "invalid number of bytes"),
        };
        let n = if suffix.is_alphabetic() {
            match strategy_param
                .iter()
                .cloned()
                .collect::<String>()
                .parse::<usize>()
            {
                Ok(a) => a,
                Err(e) => crash!(1, "invalid number of bytes: {}", e),
            }
        } else {
            match settings.strategy_param.parse::<usize>() {
                Ok(a) => a,
                Err(e) => crash!(1, "invalid number of bytes: {}", e),
            }
        };
        ByteSplitter {
            saved_bytes_to_write: n * multiplier,
            bytes_to_write: n * multiplier,
            break_on_line_end: settings.strategy == "b",
            require_whole_line: false,
        }
    }
}

impl Splitter for ByteSplitter {
    fn consume(&mut self, control: &mut SplitControl) -> String {
        let line = control.current_line.clone();
        let n = std::cmp::min(line.chars().count(), self.bytes_to_write);
        if self.require_whole_line && n < line.chars().count() {
            self.bytes_to_write = self.saved_bytes_to_write;
            control.request_new_file = true;
            self.require_whole_line = false;
            return "".to_owned();
        }
        self.bytes_to_write -= n;
        if n == 0 {
            self.bytes_to_write = self.saved_bytes_to_write;
            control.request_new_file = true;
        }
        if self.break_on_line_end && n == line.chars().count() {
            self.require_whole_line = self.break_on_line_end;
        }
        line[..n].to_owned()
    }
}

// (1, 3) -> "aab"
#[allow(clippy::many_single_char_names)]
fn str_prefix(i: usize, width: usize) -> String {
    let mut c = "".to_owned();
    let mut n = i;
    let mut w = width;
    while w > 0 {
        w -= 1;
        let div = 26usize.pow(w as u32);
        let r = n / div;
        n -= r * div;
        c.push(char::from_u32((r as u32) + 97).unwrap());
    }
    c
}

// (1, 3) -> "001"
#[allow(clippy::many_single_char_names)]
fn num_prefix(i: usize, width: usize) -> String {
    let mut c = "".to_owned();
    let mut n = i;
    let mut w = width;
    while w > 0 {
        w -= 1;
        let div = 10usize.pow(w as u32);
        let r = n / div;
        n -= r * div;
        c.push(char::from_digit(r as u32, 10).unwrap());
    }
    c
}

/// A writer that writes to a shell_process' stdin
struct FilterWriter {
    /// Running shell process
    shell_process: Child,
}

impl Write for FilterWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.shell_process
            .stdin
            .as_mut()
            .expect("failed to get shell stdin")
            .write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        self.shell_process
            .stdin
            .as_mut()
            .expect("failed to get shell stdin")
            .flush()
    }
}

impl FilterWriter {
    /// Create a new filter running a command with $FILE pointing at the output name
    ///
    /// #Arguments
    ///
    /// * `command` - The shell command to execute
    /// * `filepath` - Path of the output file (forwarded to command as $FILE)
    fn new(command: &String, filepath: &String) -> FilterWriter {
        let shell_command = match env::var("SHELL") {
            Ok(shell) => shell,
            Err(_) => String::from("/bin/sh"),
        };
        // set $FILE, save previous value (if there was one)
        let previous_file_env = env::var("FILE");
        env::set_var("FILE", &filepath);

        let shell_process = Command::new(shell_command)
            .arg("-c")
            .arg(command)
            .stdin(Stdio::piped())
            .spawn()
            .expect("Couldn't spawn filter command");

        // restore previous $FILE
        if let Ok(prev_file) = previous_file_env {
            env::set_var("FILE", prev_file)
        };

        FilterWriter {
            shell_process: shell_process,
        }
    }
}

impl Drop for FilterWriter {
    /// close stdin and wait on `shell_process` before dropping self
    fn drop(&mut self) {
        {
            // close stdin by dropping it
            let _stdin = self.shell_process.stdin.as_mut();
        }
        self.shell_process
            .wait()
            .expect("Couldn't wait for child process");
    }
}

fn split(settings: &Settings) -> i32 {
    let mut reader = BufReader::new(if settings.input == "-" {
        Box::new(stdin()) as Box<dyn Read>
    } else {
        let r = match File::open(Path::new(&settings.input)) {
            Ok(a) => a,
            Err(_) => crash!(
                1,
                "cannot open '{}' for reading: No such file or directory",
                settings.input
            ),
        };
        Box::new(r) as Box<dyn Read>
    });

    let mut splitter: Box<dyn Splitter> = match settings.strategy.as_ref() {
        "l" => Box::new(LineSplitter::new(settings)),
        "b" | "C" => Box::new(ByteSplitter::new(settings)),
        a => crash!(1, "strategy {} not supported", a),
    };

    let mut control = SplitControl {
        current_line: "".to_owned(), // Request new line
        request_new_file: true,      // Request new file
    };

    let mut writer = BufWriter::new(Box::new(stdout()) as Box<dyn Write>);
    let mut fileno = 0;
    loop {
        if control.current_line.chars().count() == 0 {
            match reader.read_line(&mut control.current_line) {
                Ok(0) | Err(_) => break,
                _ => {}
            }
        }
        if control.request_new_file {
            let mut filename = settings.prefix.clone();
            filename.push_str(
                if settings.numeric_suffix {
                    num_prefix(fileno, settings.suffix_length)
                } else {
                    str_prefix(fileno, settings.suffix_length)
                }
                .as_ref(),
            );
            filename.push_str(settings.additional_suffix.as_ref());
            // aquí "apunto" $FILE

            if fileno != 0 {
                crash_if_err!(1, writer.flush());
            }
            fileno += 1;
            // aquí if … escritor_a_stdin de un proceso ql … else … esta weá
            writer = match settings.filter {
                None => BufWriter::new(Box::new(
                    OpenOptions::new()
                        .write(true)
                        .create(true)
                        .open(Path::new(&filename))
                        .unwrap(),
                ) as Box<dyn Write>),

                Some(ref filter_command) => BufWriter::new(Box::new(FilterWriter::new(
                    &filter_command,
                    &filename,
                )) as Box<dyn Write>),
            };
            control.request_new_file = false;
            if settings.verbose {
                println!("creating file '{}'", filename);
            }
        }

        let consumed = splitter.consume(&mut control);
        crash_if_err!(1, writer.write_all(consumed.as_bytes()));

        let advance = consumed.chars().count();
        let clone = control.current_line.clone();
        let sl = clone;
        control.current_line = sl[advance..sl.chars().count()].to_owned();
    }
    0
}
