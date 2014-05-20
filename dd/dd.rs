#![crate_id(name="dd", vers="1.0.0", author="zvms")]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Colin Warren <me@zv.ms>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: dd (GNU coreutils) 8.21 */

#![feature(macro_rules)]

extern crate core;
extern crate getopts;
extern crate libc;
extern crate time;

use core::cmp::max;

use std::os;
use std::io;
use std::io::IoResult;
use std::io::print;

use time::precise_time_ns;

mod ddopts;
mod ddio;
#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "dd";

struct ActionSourceFlags;

impl ActionSourceFlags {
    fn from_vec<'a>(flags: &'a Vec<String>) -> ActionSourceFlags {
        ActionSourceFlags
    }
}

struct ActionSinkFlags;

impl ActionSinkFlags {
    fn from_vec<'a>(flags: &'a Vec<String>) -> ActionSinkFlags {
        ActionSinkFlags
    }
}

struct ActionConvFlags {
    fdatasync: bool,
    fsync: bool,
}

impl ActionConvFlags {
    fn from_vec<'a>(flags: &'a Vec<String>) -> ActionConvFlags {
        ActionConvFlags {
            fdatasync: flags.contains(&"fdatasync".to_string()),
            fsync: flags.contains(&"fsync".to_string()),
        }
    }
}

struct ActionOptions {
    bs: u64,
    ibs: u64,
    obs: u64,

    ifile: ddio::RawFD,
    ofile: ddio::RawFD,

    count: uint,

    skip: u64,
    seek: u64,

    iflag: Vec<String>,
    oflag: Vec<String>,
    conv: Vec<String>,
}

macro_rules! opt_bytes(
    ($opts:ident, $inp:expr, $def:expr) => (
        match $opts.get($inp) {
            Some(s) => {
                match ActionOptions::parse_human_bytes(s.as_slice()) {
                    Ok(n) => n,
                    Err(e) => crash!(1, "invalid opt {}={}: {}", $inp, s, e)
                }
            },
            None => {
                $def
            }
        }
    );
)

macro_rules! opt_uint(
    ($opts:ident, $inp:expr, $def:expr) => (
        match $opts.get($inp) {
            Some(s) => {
                match from_str::<uint>(s.as_slice()) {
                    Some(n) => n,
                    None => crash!(1, "invalid opt {}={}", $inp, s)
                }
            },
            None => {
                $def as uint
            }
        }
    );
)

macro_rules! opt_path(
    ($opts:ident, $inp:expr, $mode:expr, $def:expr) => (
        match $opts.get($inp) {
            Some(s) => {
                let path = Path::new(s.clone());
                match ddio::RawFD::open_file(&path, $mode) {
                    Ok(f) => f,
                    Err(e) => crash!(1, "failed to open {0} '{1}': {2}", $inp, s, e.desc)
                }
            },
            None => {
                $def
            }
        }
    );
)

macro_rules! opt_flags(
    ($opts:ident, $inp:expr) => (
        match $opts.get($inp) {
            Some(s) => {
                s.as_slice().split(',').map(|s| s.to_string()).collect()
            },
            None => {
                vec!()
            }
        }
    );
)

impl ActionOptions {
    pub fn from_args(args: Vec<String>) -> ActionOptions {
        let mut opts = ddopts::Opts::new();
        let mut action_opts = ActionOptions::new();

        opts.parse(args);

        action_opts.bs = opt_bytes!(opts, "bs", 0);
        action_opts.ibs = opt_bytes!(opts, "ibs", 512);
        action_opts.obs = opt_bytes!(opts, "obs", 512);

        action_opts.count = opt_uint!(opts, "count", 0);

        action_opts.skip = opt_bytes!(opts, "skip", 0);
        action_opts.seek = opt_bytes!(opts, "seek", 0);

        action_opts.ifile = opt_path!(opts, "if", io::Read, ddio::RawFD::stdin());
        action_opts.ofile = opt_path!(opts, "of", io::Write, ddio::RawFD::stdout());

        action_opts.iflag = opt_flags!(opts, "iflag");
        action_opts.oflag = opt_flags!(opts, "oflag");
        action_opts.conv = opt_flags!(opts, "conv");

        if !opts.ok() {
            for error in opts.errors().iter() {
                println!("dd: {}", error);
                crash!(1, "invalid parameters");
            }
        }

        action_opts
    }

    pub fn new() -> ActionOptions {
        ActionOptions {
            bs: 0,
            ibs: 512,
            obs: 512,

            ifile: ddio::RawFD::stdin(),
            ofile: ddio::RawFD::stdout(),

            seek: 0,
            skip: 0,

            count: 0,

            iflag: vec!(),
            oflag: vec!(),
            conv: vec!(),
        }
    }

    pub fn into_action<'a>(&'a mut self) -> Action<'a> {
        // Read and write block sizes
        let buffer_size = if self.bs > 0 {
            self.bs
        } else {
            max(self.ibs, self.obs)
        };

        Action {
            // Buffer sizes
            buffer_size: buffer_size,

            // Input/output
            source: &mut self.ifile,
            sink: &mut self.ofile,

            // Skip/seek
            skip: self.skip,
            seek: self.seek,

            // Count
            count: self.count,

            // Flags
            source_flags: ActionSourceFlags::from_vec(&self.iflag),
            sink_flags: ActionSinkFlags::from_vec(&self.oflag),
            conv_flags: ActionConvFlags::from_vec(&self.conv),
        }
    }

    /// Parse strings in the form XXXMB
    /// Supported suffixes:
    ///   * c  (?)         = 1
    ///   * w  (x86 word)  = 2
    ///   * b  (block)     = 512
    ///   * kB (kilobyte)  = 1000
    ///   * K  (kibibyte)  = 1024
    ///   * MB (megabyte)  = kB * 1000
    ///   * M  (mebibyte)  = K  * 1024
    ///   * xM (mebibyte)  = M
    ///   * GB (gigabyte)  = MB * 1000
    ///   * G  (gibibyte)  = M  * 1024
    ///   * TB (terabyte)  = GB * 1000
    ///   * T  (tebibyte)  = G  * 1024
    ///   * PB (petabyte)  = TB * 1000
    ///   * P  (pebibyte)  = T  * 1024
    ///   * EB (exabyte)   = PB * 1000
    ///   * E  (exbibyte)  = P  * 1024
    ///   * ZB (zettabyte) = EB * 1000
    ///   * Z  (zebibyte)  = E  * 1024
    ///   * YB (yottabyte) = ZB * 1000
    ///   * Y  (yobibyte)  = Z  * 1024
    fn parse_human_bytes(bytes: &str) -> Result<u64, String> {
        let num_str = bytes.chars().take_while(|c| c.is_digit()).collect::<String>();
        let suffix = bytes.chars().skip(num_str.len()).collect::<String>();

        if num_str.len() == 0 {
            return Err("invalid bytes".to_string());
        }

        let multiplier = match suffix.as_slice() {
            "" => 1,
            "c" => 1,
            "w" => 2,
            "b" => 512,
            "kB" => 1000,
            "K" => 1024,
            "MB" => 1000 * 1000,
            "M" => 1024 * 1024,
            "xM" => 1024 * 1024,
            "GB" => 1000 * 1000 * 1000,
            "G" => 1024 * 1024 * 1024,
            "TB" => 1000 * 1000 * 1000 * 1000,
            "T" => 1024 * 1024 * 1024 * 1024,
            "PB" => 1000 * 1000 * 1000 * 1000 * 1000,
            "P" => 1024 * 1024 * 1024 * 1024 * 1024,
            "EB" => 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
            "E" => 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
            "ZB" => 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
            "Z" => 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
            "YB" => 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
            "Y" => 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
            _ => return Err("invalid byte multiple".to_string())
        };

        match from_str(num_str.as_slice()).and_then(|num: u64| Some(num * multiplier)) {
            Some(n) => Ok(n),
            None => fail!("BUG: failed to parse number. Please file a bug report with the command line.")
        }
    }

    pub fn to_human_bytes(bytes: u64) -> String {
        let mut s = String::new();

        let possible_suffixes = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
        let mut i = 0;
        let mut human_bytes = bytes as f64;
        while human_bytes > 1000.0 && i < possible_suffixes.len() {
            i += 1;
            human_bytes /= 1000.0;
        }

        s = s.append(human_bytes.to_str().as_slice());
        s = s.append(" ");
        s = s.append(possible_suffixes[i]);

        s
    }
}

struct Action<'a> {
    // Buffer size (bytes)
    buffer_size: u64,

    // Input and output
    source: &'a mut ddio::RawFD, // if=
    sink: &'a mut ddio::RawFD, // of=

    // Skip/seek
    skip: u64,
    seek: u64,

    // Length
    count: uint, // count=

    // Flags
    source_flags: ActionSourceFlags,
    sink_flags: ActionSinkFlags,
    conv_flags: ActionConvFlags,
}

impl<'a> Action<'a> {
    pub fn execute(&mut self) -> IoResult<u64> {
        let mut buf = Vec::with_capacity(self.buffer_size as uint);
        let mut transferred: u64 = 0;
        let mut records_left = self.count;
        let skip_count = records_left == 0;

        // Seek (output)
        if self.seek > 0 {
            try!(self.sink.seek(self.seek as i64));
        }

        // Skip (input)
        if self.skip > 0 {
            try!(self.source.seek(self.skip as i64));
        }

        // Set the length of the buffer to the capacity we specified earlier.
        // It will be filled with junk data but it will be filled before it's
        // read.
        unsafe {
            buf.set_len(self.buffer_size as uint);
        }

        // The main loop
        while skip_count || records_left > 0 {
            let read = match self.source.read(buf.as_mut_slice()) {
                Ok(r) => r,
                Err(io::IoError { kind: io::EndOfFile, .. }) => return Ok(transferred),
                Err(e) => return Err(e),
            };

            try!(self.sink.write(buf.slice_to(read as uint)));

            transferred += read as u64;
            if !skip_count {
                records_left -= 1;
            }
        }

        // fdatasync - sync data to disk
        if self.conv_flags.fdatasync {
            try!(self.sink.datasync());
        }

        // fsync - sync data and metadata to disk
        if self.conv_flags.fsync {
            try!(self.sink.fsync());
        }

        Ok(transferred)
    }
}

fn main() {
    let args: Vec<String> = os::args();
    let program = args.get(0).clone();
    let opts = ~[
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            crash!(1, "invalid options\n{}", f.to_err_msg())
        }
    };

    if matches.opt_present("help") {
        println!("dd 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {0:s} [OPTIONS]...", program);
        println!("");
        print(getopts::usage(" [FILE].", opts).as_slice());
        return;
    }

    if matches.opt_present("version") {
        println!("dd 1.0.0");
        return;
    }

    let mut action_opts = ActionOptions::from_args(matches.free);
    let mut action = action_opts.into_action();

    let start_time = precise_time_ns();
    let result = action.execute();
    let end_time = precise_time_ns();

    let ns = end_time - start_time;

    match result {
        Ok(t) => {
            let s = (ns as f64)/1000000000.0;
            let th = ActionOptions::to_human_bytes(t);
            let ps = ActionOptions::to_human_bytes((t as f64 / s) as u64);

            println!("{}+0 records in", t/action.buffer_size);
            println!("{}+0 records out", t/action.buffer_size);
            println!("{} ({}) bytes copied, {} s, {}/s", t, th, s, ps);
        },
        Err(e) => crash!(1, "I/O error: {}", e.desc)
    };
}
