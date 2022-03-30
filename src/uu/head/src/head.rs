//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (vars) zlines BUFWRITER seekable

use clap::{crate_version, Arg, ArgMatches, Command};
use std::convert::{TryFrom, TryInto};
use std::ffi::OsString;
use std::io::{self, BufWriter, ErrorKind, Read, Seek, SeekFrom, Write};
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult, USimpleError};
use uucore::lines::lines;
use uucore::{format_usage, show};

const BUF_SIZE: usize = 65536;

/// The capacity in bytes for buffered writers.
const BUFWRITER_CAPACITY: usize = 16_384; // 16 kilobytes

const ABOUT: &str = "\
                     Print the first 10 lines of each FILE to standard output.\n\
                     With more than one FILE, precede each with a header giving the file name.\n\
                     With no FILE, or when FILE is -, read standard input.\n\
                     \n\
                     Mandatory arguments to long flags are mandatory for short flags too.\
                     ";
const USAGE: &str = "{} [FLAG]... [FILE]...";

mod options {
    pub const BYTES_NAME: &str = "BYTES";
    pub const LINES_NAME: &str = "LINES";
    pub const QUIET_NAME: &str = "QUIET";
    pub const VERBOSE_NAME: &str = "VERBOSE";
    pub const ZERO_NAME: &str = "ZERO";
    pub const FILES_NAME: &str = "FILE";
}
mod parse;
mod take;
use take::take_all_but;
use take::take_lines;

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::BYTES_NAME)
                .short('c')
                .long("bytes")
                .value_name("[-]NUM")
                .takes_value(true)
                .help(
                    "\
                     print the first NUM bytes of each file;\n\
                     with the leading '-', print all but the last\n\
                     NUM bytes of each file\
                     ",
                )
                .overrides_with_all(&[options::BYTES_NAME, options::LINES_NAME])
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(options::LINES_NAME)
                .short('n')
                .long("lines")
                .value_name("[-]NUM")
                .takes_value(true)
                .help(
                    "\
                     print the first NUM lines instead of the first 10;\n\
                     with the leading '-', print all but the last\n\
                     NUM lines of each file\
                     ",
                )
                .overrides_with_all(&[options::LINES_NAME, options::BYTES_NAME])
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(options::QUIET_NAME)
                .short('q')
                .long("quiet")
                .visible_alias("silent")
                .help("never print headers giving file names")
                .overrides_with_all(&[options::VERBOSE_NAME, options::QUIET_NAME]),
        )
        .arg(
            Arg::new(options::VERBOSE_NAME)
                .short('v')
                .long("verbose")
                .help("always print headers giving file names")
                .overrides_with_all(&[options::QUIET_NAME, options::VERBOSE_NAME]),
        )
        .arg(
            Arg::new(options::ZERO_NAME)
                .short('z')
                .long("zero-terminated")
                .help("line delimiter is NUL, not newline")
                .overrides_with(options::ZERO_NAME),
        )
        .arg(Arg::new(options::FILES_NAME).multiple_occurrences(true))
}

#[derive(Debug, PartialEq)]
enum Mode {
    FirstLines(u64),
    AllButLastLines(u64),
    FirstBytes(u64),
    AllButLastBytes(u64),
}

impl Default for Mode {
    fn default() -> Self {
        Self::FirstLines(10)
    }
}

impl Mode {
    fn from(matches: &ArgMatches) -> Result<Self, String> {
        if let Some(v) = matches.value_of(options::BYTES_NAME) {
            let (n, all_but_last) =
                parse::parse_num(v).map_err(|err| format!("invalid number of bytes: {}", err))?;
            if all_but_last {
                Ok(Self::AllButLastBytes(n))
            } else {
                Ok(Self::FirstBytes(n))
            }
        } else if let Some(v) = matches.value_of(options::LINES_NAME) {
            let (n, all_but_last) =
                parse::parse_num(v).map_err(|err| format!("invalid number of lines: {}", err))?;
            if all_but_last {
                Ok(Self::AllButLastLines(n))
            } else {
                Ok(Self::FirstLines(n))
            }
        } else {
            Ok(Default::default())
        }
    }
}

fn arg_iterate<'a>(
    mut args: impl uucore::Args + 'a,
) -> Result<Box<dyn Iterator<Item = OsString> + 'a>, String> {
    // argv[0] is always present
    let first = args.next().unwrap();
    if let Some(second) = args.next() {
        if let Some(s) = second.to_str() {
            match parse::parse_obsolete(s) {
                Some(Ok(iter)) => Ok(Box::new(vec![first].into_iter().chain(iter).chain(args))),
                Some(Err(e)) => match e {
                    parse::ParseError::Syntax => Err(format!("bad argument format: {}", s.quote())),
                    parse::ParseError::Overflow => Err(format!(
                        "invalid argument: {} Value too large for defined datatype",
                        s.quote()
                    )),
                },
                None => Ok(Box::new(vec![first, second].into_iter().chain(args))),
            }
        } else {
            Err("bad argument encoding".to_owned())
        }
    } else {
        Ok(Box::new(vec![first].into_iter()))
    }
}

#[derive(Debug, PartialEq, Default)]
struct HeadOptions {
    pub quiet: bool,
    pub verbose: bool,
    pub zeroed: bool,
    pub mode: Mode,
    pub files: Vec<String>,
}

impl HeadOptions {
    ///Construct options from matches
    pub fn get_from(args: impl uucore::Args) -> Result<Self, String> {
        let matches = uu_app().get_matches_from(arg_iterate(args)?);

        let mut options = Self::default();

        options.quiet = matches.is_present(options::QUIET_NAME);
        options.verbose = matches.is_present(options::VERBOSE_NAME);
        options.zeroed = matches.is_present(options::ZERO_NAME);

        options.mode = Mode::from(&matches)?;

        options.files = match matches.values_of(options::FILES_NAME) {
            Some(v) => v.map(|s| s.to_owned()).collect(),
            None => vec!["-".to_owned()],
        };
        //println!("{:#?}", options);
        Ok(options)
    }
}

fn read_n_bytes<R>(input: R, n: u64) -> std::io::Result<()>
where
    R: Read,
{
    // Read the first `n` bytes from the `input` reader.
    let mut reader = input.take(n);

    // Write those bytes to `stdout`.
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    io::copy(&mut reader, &mut stdout)?;

    Ok(())
}

fn read_n_lines(input: &mut impl std::io::BufRead, n: u64, zero: bool) -> std::io::Result<()> {
    // Read the first `n` lines from the `input` reader.
    let separator = if zero { b'\0' } else { b'\n' };
    let mut reader = take_lines(input, n, separator);

    // Write those bytes to `stdout`.
    let stdout = std::io::stdout();
    let stdout = stdout.lock();
    let mut writer = BufWriter::with_capacity(BUFWRITER_CAPACITY, stdout);

    io::copy(&mut reader, &mut writer)?;

    Ok(())
}

fn read_but_last_n_bytes(input: &mut impl std::io::BufRead, n: usize) -> std::io::Result<()> {
    if n == 0 {
        //prints everything
        return read_n_bytes(input, std::u64::MAX);
    }

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    let mut ring_buffer = vec![0u8; n];

    // first we fill the ring buffer
    if let Err(e) = input.read_exact(&mut ring_buffer) {
        if e.kind() == ErrorKind::UnexpectedEof {
            return Ok(());
        } else {
            return Err(e);
        }
    }
    let mut buffer = [0u8; BUF_SIZE];
    loop {
        let read = loop {
            match input.read(&mut buffer) {
                Ok(n) => break n,
                Err(e) => match e.kind() {
                    ErrorKind::Interrupted => {}
                    _ => return Err(e),
                },
            }
        };
        if read == 0 {
            return Ok(());
        } else if read >= n {
            stdout.write_all(&ring_buffer)?;
            stdout.write_all(&buffer[..read - n])?;
            for i in 0..n {
                ring_buffer[i] = buffer[read - n + i];
            }
        } else {
            stdout.write_all(&ring_buffer[..read])?;
            for i in 0..n - read {
                ring_buffer[i] = ring_buffer[read + i];
            }
            ring_buffer[n - read..].copy_from_slice(&buffer[..read]);
        }
    }
}

fn read_but_last_n_lines(
    input: impl std::io::BufRead,
    n: usize,
    zero: bool,
) -> std::io::Result<()> {
    if zero {
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();
        for bytes in take_all_but(lines(input, b'\0'), n) {
            stdout.write_all(&bytes?)?;
        }
    } else {
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();
        for bytes in take_all_but(lines(input, b'\n'), n) {
            stdout.write_all(&bytes?)?;
        }
    }
    Ok(())
}

/// Return the index in `input` just after the `n`th line from the end.
///
/// If `n` exceeds the number of lines in this file, then return 0.
///
/// The cursor must be at the start of the seekable input before
/// calling this function. This function rewinds the cursor to the
/// beginning of the input just before returning unless there is an
/// I/O error.
///
/// If `zeroed` is `false`, interpret the newline character `b'\n'` as
/// a line ending. If `zeroed` is `true`, interpret the null character
/// `b'\0'` as a line ending instead.
///
/// # Errors
///
/// This function returns an error if there is a problem seeking
/// through or reading the input.
///
/// # Examples
///
/// The function returns the index of the byte immediately following
/// the line ending character of the `n`th line from the end of the
/// input:
///
/// ```rust,ignore
/// let mut input = Cursor::new("x\ny\nz\n");
/// assert_eq!(find_nth_line_from_end(&mut input, 0, false).unwrap(), 6);
/// assert_eq!(find_nth_line_from_end(&mut input, 1, false).unwrap(), 4);
/// assert_eq!(find_nth_line_from_end(&mut input, 2, false).unwrap(), 2);
/// ```
///
/// If `n` exceeds the number of lines in the file, always return 0:
///
/// ```rust,ignore
/// let mut input = Cursor::new("x\ny\nz\n");
/// assert_eq!(find_nth_line_from_end(&mut input, 3, false).unwrap(), 0);
/// assert_eq!(find_nth_line_from_end(&mut input, 4, false).unwrap(), 0);
/// assert_eq!(find_nth_line_from_end(&mut input, 1000, false).unwrap(), 0);
/// ```
fn find_nth_line_from_end<R>(input: &mut R, n: u64, zeroed: bool) -> std::io::Result<u64>
where
    R: Read + Seek,
{
    let size = input.seek(SeekFrom::End(0))?;

    let mut buffer = [0u8; BUF_SIZE];
    let buf_size: usize = (BUF_SIZE as u64).min(size).try_into().unwrap();
    let buffer = &mut buffer[..buf_size];

    let mut i = 0u64;
    let mut lines = 0u64;

    loop {
        // the casts here are ok, `buffer.len()` should never be above a few k
        input.seek(SeekFrom::Current(
            -((buffer.len() as i64).min((size - i) as i64)),
        ))?;
        input.read_exact(buffer)?;
        for byte in buffer.iter().rev() {
            match byte {
                b'\n' if !zeroed => {
                    lines += 1;
                }
                0u8 if zeroed => {
                    lines += 1;
                }
                _ => {}
            }
            // if it were just `n`,
            if lines == n + 1 {
                input.seek(SeekFrom::Start(0))?;
                return Ok(size - i);
            }
            i += 1;
        }
        if size - i == 0 {
            input.seek(SeekFrom::Start(0))?;
            return Ok(0);
        }
    }
}

fn head_backwards_file(input: &mut std::fs::File, options: &HeadOptions) -> std::io::Result<()> {
    match options.mode {
        Mode::AllButLastBytes(n) => {
            let size = input.metadata()?.len();
            if n >= size {
                return Ok(());
            } else {
                read_n_bytes(
                    &mut std::io::BufReader::with_capacity(BUF_SIZE, input),
                    size - n,
                )?;
            }
        }
        Mode::AllButLastLines(n) => {
            let found = find_nth_line_from_end(input, n, options.zeroed)?;
            read_n_bytes(
                &mut std::io::BufReader::with_capacity(BUF_SIZE, input),
                found,
            )?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn head_file(input: &mut std::fs::File, options: &HeadOptions) -> std::io::Result<()> {
    match options.mode {
        Mode::FirstBytes(n) => {
            read_n_bytes(&mut std::io::BufReader::with_capacity(BUF_SIZE, input), n)
        }
        Mode::FirstLines(n) => read_n_lines(
            &mut std::io::BufReader::with_capacity(BUF_SIZE, input),
            n,
            options.zeroed,
        ),
        Mode::AllButLastBytes(_) | Mode::AllButLastLines(_) => head_backwards_file(input, options),
    }
}

fn uu_head(options: &HeadOptions) -> UResult<()> {
    let mut first = true;
    for file in &options.files {
        let res = match file.as_str() {
            "-" => {
                if (options.files.len() > 1 && !options.quiet) || options.verbose {
                    if !first {
                        println!();
                    }
                    println!("==> standard input <==");
                }
                let stdin = std::io::stdin();
                let mut stdin = stdin.lock();

                // Outputting "all-but-last" requires us to use a ring buffer with size n, so n
                // must be converted from u64 to usize to fit in memory. If such conversion fails,
                // it means the platform doesn't have enough memory to hold the buffer, so we fail.
                if let Mode::AllButLastLines(n) | Mode::AllButLastBytes(n) = options.mode {
                    if let Err(e) = usize::try_from(n) {
                        show!(USimpleError::new(
                            1,
                            format!("{}: number of bytes is too large", e)
                        ));
                        continue;
                    };
                };

                match options.mode {
                    Mode::FirstBytes(n) => read_n_bytes(&mut stdin, n),
                    // unwrap is guaranteed to succeed because we checked the value of n above
                    Mode::AllButLastBytes(n) => {
                        read_but_last_n_bytes(&mut stdin, n.try_into().unwrap())
                    }
                    Mode::FirstLines(n) => read_n_lines(&mut stdin, n, options.zeroed),
                    // unwrap is guaranteed to succeed because we checked the value of n above
                    Mode::AllButLastLines(n) => {
                        read_but_last_n_lines(&mut stdin, n.try_into().unwrap(), options.zeroed)
                    }
                }
            }
            name => {
                let mut file = match std::fs::File::open(name) {
                    Ok(f) => f,
                    Err(err) => {
                        show!(err.map_err_context(|| format!(
                            "cannot open {} for reading",
                            name.quote()
                        )));
                        continue;
                    }
                };
                if (options.files.len() > 1 && !options.quiet) || options.verbose {
                    if !first {
                        println!();
                    }
                    println!("==> {} <==", name);
                }
                head_file(&mut file, options)
            }
        };
        if res.is_err() {
            let name = if file.as_str() == "-" {
                "standard input"
            } else {
                file
            };
            show!(USimpleError::new(
                1,
                format!("error reading {}: Input/output error", name)
            ));
        }
        first = false;
    }
    // Even though this is returning `Ok`, it is possible that a call
    // to `show!()` and thus a call to `set_exit_code()` has been
    // called above. If that happens, then this process will exit with
    // a non-zero exit code.
    Ok(())
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = match HeadOptions::get_from(args) {
        Ok(o) => o,
        Err(s) => {
            return Err(USimpleError::new(1, s));
        }
    };
    uu_head(&args)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::io::Cursor;

    use super::*;
    fn options(args: &str) -> Result<HeadOptions, String> {
        let combined = "head ".to_owned() + args;
        let args = combined.split_whitespace();
        HeadOptions::get_from(args.map(OsString::from))
    }
    #[test]
    fn test_args_modes() {
        let args = options("-n -10M -vz").unwrap();
        assert!(args.zeroed);
        assert!(args.verbose);
        assert_eq!(args.mode, Mode::AllButLastLines(10 * 1024 * 1024));
    }
    #[test]
    fn test_gnu_compatibility() {
        let args = options("-n 1 -c 1 -n 5 -c kiB -vqvqv").unwrap(); // spell-checker:disable-line
        assert!(args.mode == Mode::FirstBytes(1024));
        assert!(args.verbose);
        assert_eq!(options("-5").unwrap().mode, Mode::FirstLines(5));
        assert_eq!(options("-2b").unwrap().mode, Mode::FirstBytes(1024));
        assert_eq!(options("-5 -c 1").unwrap().mode, Mode::FirstBytes(1));
    }
    #[test]
    fn all_args_test() {
        assert!(options("--silent").unwrap().quiet);
        assert!(options("--quiet").unwrap().quiet);
        assert!(options("-q").unwrap().quiet);
        assert!(options("--verbose").unwrap().verbose);
        assert!(options("-v").unwrap().verbose);
        assert!(options("--zero-terminated").unwrap().zeroed);
        assert!(options("-z").unwrap().zeroed);
        assert_eq!(options("--lines 15").unwrap().mode, Mode::FirstLines(15));
        assert_eq!(options("-n 15").unwrap().mode, Mode::FirstLines(15));
        assert_eq!(options("--bytes 15").unwrap().mode, Mode::FirstBytes(15));
        assert_eq!(options("-c 15").unwrap().mode, Mode::FirstBytes(15));
    }
    #[test]
    fn test_options_errors() {
        assert!(options("-n IsThisTheRealLife?").is_err());
        assert!(options("-c IsThisJustFantasy").is_err());
    }
    #[test]
    fn test_options_correct_defaults() {
        let opts: HeadOptions = Default::default();

        assert!(!opts.verbose);
        assert!(!opts.quiet);
        assert!(!opts.zeroed);
        assert_eq!(opts.mode, Mode::FirstLines(10));
        assert!(opts.files.is_empty());
    }
    fn arg_outputs(src: &str) -> Result<String, String> {
        let split = src.split_whitespace().map(OsString::from);
        match arg_iterate(split) {
            Ok(args) => {
                let vec = args
                    .map(|s| s.to_str().unwrap().to_owned())
                    .collect::<Vec<_>>();
                Ok(vec.join(" "))
            }
            Err(e) => Err(e),
        }
    }
    #[test]
    fn test_arg_iterate() {
        // test that normal args remain unchanged
        assert_eq!(
            arg_outputs("head -n -5 -zv"),
            Ok("head -n -5 -zv".to_owned())
        );
        // tests that nonsensical args are unchanged
        assert_eq!(
            arg_outputs("head -to_be_or_not_to_be,..."),
            Ok("head -to_be_or_not_to_be,...".to_owned())
        );
        //test that the obsolete syntax is unrolled
        assert_eq!(
            arg_outputs("head -123qvqvqzc"), // spell-checker:disable-line
            Ok("head -q -z -c 123".to_owned())
        );
        //test that bad obsoletes are an error
        assert!(arg_outputs("head -123FooBar").is_err());
        //test overflow
        assert!(arg_outputs("head -100000000000000000000000000000000000000000").is_err());
        //test that empty args remain unchanged
        assert_eq!(arg_outputs("head"), Ok("head".to_owned()));
    }
    #[test]
    #[cfg(target_os = "linux")]
    fn test_arg_iterate_bad_encoding() {
        let invalid = unsafe { std::str::from_utf8_unchecked(b"\x80\x81") };
        // this arises from a conversion from OsString to &str
        assert!(
            arg_iterate(vec![OsString::from("head"), OsString::from(invalid)].into_iter()).is_err()
        );
    }
    #[test]
    fn read_early_exit() {
        let mut empty = std::io::BufReader::new(std::io::Cursor::new(Vec::new()));
        assert!(read_n_bytes(&mut empty, 0).is_ok());
        assert!(read_n_lines(&mut empty, 0, false).is_ok());
    }

    #[test]
    fn test_find_nth_line_from_end() {
        let mut input = Cursor::new("x\ny\nz\n");
        assert_eq!(find_nth_line_from_end(&mut input, 0, false).unwrap(), 6);
        assert_eq!(find_nth_line_from_end(&mut input, 1, false).unwrap(), 4);
        assert_eq!(find_nth_line_from_end(&mut input, 2, false).unwrap(), 2);
        assert_eq!(find_nth_line_from_end(&mut input, 3, false).unwrap(), 0);
        assert_eq!(find_nth_line_from_end(&mut input, 4, false).unwrap(), 0);
        assert_eq!(find_nth_line_from_end(&mut input, 1000, false).unwrap(), 0);
    }
}
