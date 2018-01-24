#![crate_name = "uu_dd"]

#[macro_use]
extern crate chan;
extern crate chan_signal;

#[macro_use]
extern crate uucore;

use std::cmp;
use std::error::Error;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, Write};
use std::process::exit;
use std::sync::{Arc, RwLock};
use std::thread;

use chan_signal::Signal;

const BUF_SIZE: usize = 4096;

pub struct Conf {
    inp_name: String,
    out_name: String,
    is_stdin: bool,
    is_stdout: bool,

    inp_block_size: usize,
    out_block_size: usize,
    cnv_block_size: usize,

    skip: usize,
    seek: usize,

    count: Option<usize>,

    inp_part: usize,
    inp_full: usize,
    out_part: usize,
    out_full: usize,
    trunc: usize,

    flags: u32,

    short_to_short: bool,
}

impl Default for Conf {
    fn default() -> Self {
        Conf {
            inp_name: "".to_string(),
            out_name: "".to_string(),
            is_stdin: true,
            is_stdout: true,

            inp_block_size: Self::DEFAULT_BLOCK_SIZE,
            out_block_size: Self::DEFAULT_BLOCK_SIZE,
            cnv_block_size: 0,

            skip: 0,
            seek: 0,

            count: None,

            inp_part: 0,
            inp_full: 0,
            out_part: 0,
            out_full: 0,
            trunc: 0,

            flags: 0,

            short_to_short: false,
        }
    }
}

type FlagT = u32;

impl Conf {
    const DEFAULT_BLOCK_SIZE: usize = 512;

    pub fn set_if(&mut self, input_filename: &str) {
        self.inp_name = input_filename.to_string();
        self.is_stdin = false;
    }

    pub fn set_of(&mut self, output_filename: &str) {
        self.out_name = output_filename.to_string();
        self.is_stdout = false;
    }

    pub fn set_flag(&mut self, flag: FlagT) {
        self.flags |= flag;
    }

    pub fn get_flag(flag: &str) -> FlagT {
        match flag {
            "block" => Conf::BLOCK,
            "unblock" => Conf::UNBLOCK,
            "lcase" => Conf::LCASE,
            "ucase" => Conf::UCASE,
            "swab" => Conf::SWAB,
            "noerror" => Conf::NOERROR,
            "notrunc" => Conf::NOTRUNC,
            "sync" => Conf::SYNC,
            _ => Conf::INVALID_FLAG,
        }
    }

    pub fn is_flag_valid(flag: &str) -> bool {
        match flag {
            "ibs" | "obs" | "bs" | "cbs" | "if" | "of" | "skip" | "seek" | "count" => true,
            _ => false,
        }
    }

    pub fn has_flag(&self, flag: FlagT) -> bool {
        if self.flags & flag != 0 {
            true
        } else {
            false
        }
    }

    pub fn str_flags(&self) -> String {
        let mut result = String::new();
        if self.flags & Self::BLOCK != 0 {
            result += "[block]";
        }
        if self.flags & Self::UNBLOCK != 0 {
            result += "[unblock]";
        }
        if self.flags & Self::LCASE != 0 {
            result += "[lcase]";
        }
        if self.flags & Self::UCASE != 0 {
            result += "[ucase]";
        }
        if self.flags & Self::SWAB != 0 {
            result += "[swab]";
        }
        if self.flags & Self::NOERROR != 0 {
            result += "[noerror]";
        }
        if self.flags & Self::NOTRUNC != 0 {
            result += "[notrunc]";
        }
        if self.flags & Self::SYNC != 0 {
            result += "[sync]";
        }
        result
    }

    const BLOCK: FlagT = 0b00000001;
    const UNBLOCK: FlagT = 0b00000010;
    const LCASE: FlagT = 0b00000100;
    const UCASE: FlagT = 0b00001000;
    const SWAB: FlagT = 0b00010000;
    const NOERROR: FlagT = 0b00100000;
    const NOTRUNC: FlagT = 0b01000000;
    const SYNC: FlagT = 0b10000000;
    const INVALID_FLAG: FlagT = 0b1_00000000;
}

impl fmt::Display for Conf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let count = self.count.unwrap().to_string();
        write!(
            f,
            "if: {input_filename}\n\
             of: {output_filename}\n\
             ibs: {ibs}\n\
             obs: {obs}\n\
             cbs: {cbs}\n\
             skip: {skip}\n\
             seek: {seek}\n\
             count: {count}\n\
             flags: {flags}\n",
            input_filename = self.inp_name,
            output_filename = self.out_name,
            ibs = self.inp_block_size,
            obs = self.out_block_size,
            cbs = self.cnv_block_size,
            skip = self.skip,
            seek = self.seek,
            count = if self.count.is_some() {
                count.as_str()
            } else {
                "wasn't specified"
            },
            flags = self.str_flags()
        )
    }
}

pub fn parse_args(args: Vec<String>) -> Conf {
    let mut cf = Conf::default();
    for arg in args.into_iter().skip(1) {
        let pair: Vec<&str> = arg.splitn(2, '=').collect();
        if pair.len() == 2 {
            let (key, val) = (pair[0], pair[1]);
            match key {
                "if" => cf.set_if(val),
                "of" => cf.set_of(val),
                "conv" => for elem in val.split(",") {
                    match Conf::get_flag(elem) {
                        Conf::INVALID_FLAG => err("invalid flag passed to conv"),
                        flag => cf.set_flag(flag),
                    }
                },
                _ => {
                    if Conf::is_flag_valid(key) {
                        if let Some(m) = get_u(val) {
                            match key {
                                "ibs" => cf.inp_block_size = m,
                                "obs" => cf.out_block_size = m,
                                "bs" => {
                                    cf.inp_block_size = m;
                                    cf.out_block_size = m;
                                }
                                "cbs" => cf.cnv_block_size = m,
                                "skip" => cf.skip = m,
                                "seek" => cf.seek = m,
                                "count" => if m == 0 {
                                    err("assign count non-zero value");
                                } else {
                                    cf.count = Some(m)
                                },
                                _ => err(format!("invalid argument: {}", key)),
                            }
                        }
                    } else {
                        err(format!("invalid argument: {}", arg));
                    }
                }
            }
        } else {
            err(format!("invalid argument: {}", arg));
        }
    }

    // Behaviour is unspecified. We choose to terminate.
    if cf.count.is_none() {
        err("pass count with non-zero value");
    }

    if cf.is_stdout && cf.seek != 0 {
        err("cannot seek on standard output");
    }

    if cf.has_flag(Conf::BLOCK) && cf.has_flag(Conf::UNBLOCK) {
        err("'block' and 'unblock' values are mutually exclusive");
    }

    if cf.has_flag(Conf::LCASE) && cf.has_flag(Conf::UCASE) {
        err("'lcase' and 'ucase' values are mutually exclusive");
    }

    if !(cf.flags & Conf::BLOCK != 0 || cf.flags & Conf::UNBLOCK != 0 || cf.flags & Conf::LCASE != 0
        || cf.flags & Conf::UCASE != 0 || cf.flags & Conf::SWAB != 0)
    {
        cf.short_to_short = true
    }

    // Behaviour is unspecified. We choose to terminate.
    if cf.cnv_block_size == 0 && (cf.has_flag(Conf::BLOCK) || cf.has_flag(Conf::UNBLOCK)) {
        err("assign cbs non-zero value");
    }

    cf
}

fn setup_input(cf: Arc<RwLock<Conf>>) -> InpFileWrap {
    let cf_guard = cf.write().unwrap();

    let inp = if cf_guard.is_stdin {
        Some(InpFileWrap {
            f: InpFileType::Stdin(io::stdin()), // TODO: use .lock() if necessary
            is_seekable: false,
        })
    } else {
        match File::open(&cf_guard.inp_name) {
            Err(e) => err(format!("cannot open input file: {}", e.description())),
            Ok(f) => {
                Some(InpFileWrap {
                    f: InpFileType::Reg(f),
                    is_seekable: true, // TODO: handle case when device isn't seekable
                })
            }
        }
    };
    let mut inp = inp.unwrap();

    match inp.f {
        InpFileType::Reg(ref mut f) => {
            if inp.is_seekable {
                if f.seek(io::SeekFrom::Start(
                    (cf_guard.skip * cf_guard.inp_block_size) as u64,
                )).is_err()
                {
                    err("cannot skip on input file".to_string());
                }
            } else {
                let mut to_skip = cf_guard.skip * cf_guard.inp_block_size;
                let mut buf = vec![0u8; BUF_SIZE];
                while to_skip > 0 {
                    let to_read = cmp::min(to_skip, BUF_SIZE);
                    match f.read_exact(&mut buf[..to_read]) {
                        Ok(_) => to_skip -= to_read,
                        _ => err("cannot skip on input file".to_string()),
                    }
                }
            }
        }
        InpFileType::Stdin(ref mut s) => {
            let mut to_skip = cf_guard.skip * cf_guard.inp_block_size;
            let mut buf = vec![0u8; BUF_SIZE];
            while to_skip > 0 {
                let to_read = cmp::min(to_skip, BUF_SIZE);
                match s.read_exact(&mut buf[..to_read]) {
                    Ok(_) => to_skip -= to_read,
                    _ => err("cannot skip on input file".to_string()),
                }
            }
        }
    }
    inp
}

fn setup_output(cf: Arc<RwLock<Conf>>) -> OutFileWrap {
    let cf_guard = cf.write().unwrap();

    let out = if cf_guard.is_stdout {
        Some(OutFileWrap {
            f: OutFileType::Stdout(io::stdout()), // TODO: use .lock() if necessary
            is_seekable: false,
        })
    } else {
        match File::create(&cf_guard.out_name) {
            Err(e) => err(format!("cannot open output file: {}", e.description())),
            Ok(f) => {
                Some(OutFileWrap {
                    f: OutFileType::Reg(f),
                    is_seekable: true, // TODO: handle case when device isn't seekable
                })
            }
        }
    };
    let mut out = out.unwrap();

    let mut opts = OpenOptions::new();
    opts.read(false).write(true);

    let zero_buf: Vec<u8> = vec![0; 4096];
    match out.f {
        OutFileType::Reg(ref mut f) => {
            if out.is_seekable {
                if cf_guard.seek == 0 {
                    if !cf_guard.has_flag(Conf::NOTRUNC) {
                        if f.seek(io::SeekFrom::Start(cf_guard.seek as u64)).is_err() {
                            err("cannot seek on output file".to_string());
                        }
                    }
                }
            } else {
                let mut to_skip = cf_guard.seek * cf_guard.out_block_size;
                let mut buf = vec![0u8; BUF_SIZE];
                while to_skip > 0 {
                    match f.read(&mut buf[..cmp::min(to_skip, BUF_SIZE)]) {
                        Ok(0) => break,
                        Ok(m) => to_skip -= m,
                        _ => err("cannot seek on output file".to_string()),
                    }
                }
                if to_skip > 0 {
                    while to_skip > 0 {
                        match f.write(&zero_buf[..cmp::min(to_skip, BUF_SIZE)]) {
                            Ok(0) => err("cannot seek on output file".to_string()),
                            Ok(m) => to_skip -= m,
                            _ => err("cannot seek on output file".to_string()),
                        }
                    }
                }
            }
        }

        // Would have terminated early if seek != 0, test precondition anyway
        OutFileType::Stdout(_) => assert_eq!(cf_guard.seek, 0),
    }
    out
}

// , _sdone: chan::Sender<()>
fn setup(cf: Arc<RwLock<Conf>>) -> (InpFileWrap, OutFileWrap) {
    let inp = setup_input(Arc::clone(&cf));
    let out = setup_output(cf);

    (inp, out)
}

fn copy(
    mut inp: InpFileWrap,
    mut out: OutFileWrap,
    cf_ptr: Arc<RwLock<Conf>>,
    _sdone: chan::Sender<()>,
) {
    let (total_count, mut count, inp_block_size, out_block_size) = {
        let mut cf = cf_ptr.write().unwrap();
        let count = cf.count.unwrap(); // it's Some, checked after arguments parsing
        (count, count, cf.inp_block_size, cf.out_block_size)
    };

    let mut buf = vec![0u8; cmp::min(inp_block_size, out_block_size)];
    let mut total = 0;
    loop {
        let mut pos = 0;
        let mut left = inp_block_size;
        let mut session = 0;
        'read_block: loop {
            if pos == inp_block_size {
                break 'read_block;
            }
            let mut is_read_0 = false;
            match inp.read(&mut buf[pos..pos + left]) {
                Ok(0) => is_read_0 = true,
                Ok(num) => {
                    session += num;
                    total += num;
                    let mut cf = cf_ptr.write().unwrap();
                    if num < inp_block_size {
                        cf.inp_part += 1;
                        pos += num;
                        left = cf.inp_block_size - pos;
                    } else if num == cf.inp_block_size {
                        cf.inp_full += 1;
                    } else {
                        err(format!("overread: {} over {}", num, inp_block_size));
                    }
                }
                Err(_) => {
                    // TODO: handle
                }
            }
            if session == inp_block_size || is_read_0 {
                break 'read_block;
            }
        }
        let num = cmp::min(session, inp_block_size);
        let mut pos = 0;
        loop {
            if pos == num {
                break;
            }
            match out.write(&buf[pos..pos + num]) {
                Ok(m) => {
                    let mut cf = cf_ptr.write().unwrap();
                    if m < cf.out_block_size {
                        cf.out_part += 1;
                    } else {
                        cf.out_full += 1;
                    }
                    pos += m;
                }
                Err(e) => err(format!("error occured {}", e.description())), // TODO: handle
            }
        }
        if total >= inp_block_size * total_count {
            break;
        }
        count -= 1;
        if count == 0 {
            break;
        }
    }
    display_stat(cf_ptr);
}

enum InpFileType {
    Reg(File),
    Stdin(io::Stdin),
}

pub struct InpFileWrap {
    f: InpFileType,
    is_seekable: bool,
}

impl InpFileWrap {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.f {
            InpFileType::Reg(ref mut f) => f.read(buf),
            InpFileType::Stdin(ref mut s) => s.read(buf),
        }
    }
}

enum OutFileType {
    Reg(File),
    Stdout(io::Stdout),
}

pub struct OutFileWrap {
    f: OutFileType,
    is_seekable: bool,
}

impl OutFileWrap {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.f {
            OutFileType::Reg(ref mut f) => f.write(buf),
            OutFileType::Stdout(ref mut s) => s.write(buf),
        }
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let signal = if cfg!(target_os = "freebsd") || cfg!(target_os = "openbsd")
        || cfg!(target_os = "netbsd") || cfg!(target_os = "dragonflybsd") {
        chan_signal::notify(&[Signal::INT, Signal::USR1])
    } else { // BSD
        chan_signal::notify(&[Signal::INT, Signal::USR1])
        // No Signal::INFO yet.
        // chan_signal::notify(&[Signal::INT, Signal::INFO])
    };
    let (s_done, r_done) = chan::sync(0);

    let cf = parse_args(args);
    let cf1 = Arc::new(RwLock::new(cf));
    let cf2 = Arc::clone(&cf1);

    let handle = thread::spawn(move || {
        let (inp, out) = setup(Arc::clone(&cf1));
        copy(inp, out, cf1, s_done);
    });

    loop {
        chan_select! {
            signal.recv() -> signal => {
                display_stat(Arc::clone(&cf2));
                if let Some(Signal::INT) = signal {
                    exit(1);
                }
            },
            r_done.recv() => break
        }
    }
    let _ = handle.join();
    0
}

pub fn get_u(val: &str) -> Option<usize> {
    if val.is_empty() || (val.len() == 1 && !val.chars().nth(0).unwrap().is_digit(10))
        || val.chars().nth(0).unwrap() == '-'
    {
        return None;
    }

    let mut pref = val;
    let mut size: usize = match val.chars().nth(val.len() - 1) {
        Some('k') => {
            pref = &val[..val.len() - 1];
            1024
        }
        Some('b') => {
            pref = &val[..val.len() - 1];
            512
        }
        Some(m) if !m.is_digit(10) && m != 'k' && m != 'b' => {
            return None;
        }
        _ => 1,
    };

    if pref.is_empty() || pref.chars().nth(pref.len() - 1).unwrap() == 'x'
        || pref.chars().nth(0).unwrap() == 'x'
    {
        return None;
    }
    for m in pref.split('x') {
        if let Ok(n) = m.parse::<usize>() {
            size *= n;
        } else {
            return None;
        }
    }

    Some(size)
}

pub fn err<S: Into<String>>(s: S) -> ! {
    eprintln!("{}", s.into());
    exit(1);
}

pub fn display_stat(cf: Arc<RwLock<Conf>>) {
    let cf = cf.read().unwrap();

    eprintln!(
        "{}+{} records in\n{}+{} records out",
        cf.inp_full, cf.inp_part, cf.out_full, cf.out_part
    );

    if cf.trunc != 0 {
        eprintln!(
            "{} truncated record{}",
            cf.trunc,
            if cf.trunc == 1 { "" } else { "s" }
        );
    }
}

#[test]
fn test_get_bsz() {
    use super::util::args::get_bsz;

    assert_eq!(Some(40), get_bsz("4x2x5"));
    assert_eq!(Some(20480), get_bsz("4x2x5b"));
    assert_eq!(Some(1024), get_bsz("1k"));
    assert_eq!(Some(1536), get_bsz("3b"));
    assert_eq!(Some(2048), get_bsz("2x2b"));

    assert_eq!(None, get_bsz("-"));
    assert_eq!(None, get_bsz("-1"));
    assert_eq!(None, get_bsz("k"));
    assert_eq!(None, get_bsz("1-"));
    assert_eq!(None, get_bsz("2x2a"));
    assert_eq!(None, get_bsz("2x2K"));
    assert_eq!(None, get_bsz("x1k"));
    assert_eq!(None, get_bsz("1xk"));
    assert_eq!(None, get_bsz("2x4ix8k"));
}

#[test]
fn test_get_off() {
    use super::util::args::get_off;

    assert_eq!(Some(1), get_off("1"));
    assert_eq!(Some(-6123), get_off("-6123"));

    assert_eq!(None, get_off("1-"));
    //    assert_eq!(Some(40), get_bsz("4x2x5"));
    //    assert_eq!(Some(20480), get_bsz("4x2x5b"));
    //    assert_eq!(Some(1024), get_bsz("1k"));
    //    assert_eq!(Some(1536), get_bsz("3b"));
    //    assert_eq!(Some(2048), get_bsz("2x2b"));

    //    assert_eq!(None, get_bsz("-"));
    //    assert_eq!(None, get_bsz("-1"));
    //    assert_eq!(None, get_bsz("k"));
    //    assert_eq!(None, get_bsz("1-"));
    //    assert_eq!(None, get_bsz("2x2a"));
    //    assert_eq!(None, get_bsz("2x2K"));
    //    assert_eq!(None, get_bsz("x1k"));
    //    assert_eq!(None, get_bsz("1xk"));
    //    assert_eq!(None, get_bsz("2x4ix8k"));
}
