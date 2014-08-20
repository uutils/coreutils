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
#[path = "../common/bytes.rs"]
mod bytes;

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
    noerror: bool,
    nocreat: bool,
}

impl ActionConvFlags {
    fn from_vec<'a>(flags: &'a Vec<String>) -> ActionConvFlags {
        ActionConvFlags {
            fdatasync: flags.contains(&"fdatasync".to_string()),
            fsync: flags.contains(&"fsync".to_string()),
            noerror: flags.contains(&"noerror".to_string()),
            nocreat: flags.contains(&"nocreat".to_string())
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
                match bytes::from_human(s.as_slice()) {
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
            skip: self.skip * self.ibs,
            seek: self.seek * self.obs,

            // Count
            count: self.count,

            // Flags
            source_flags: ActionSourceFlags::from_vec(&self.iflag),
            sink_flags: ActionSinkFlags::from_vec(&self.oflag),
            conv_flags: ActionConvFlags::from_vec(&self.conv),
        }
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
                Err(e) => {
                    // noerror - ignore read errors
                    if !self.conv_flags.noerror {
                        return Err(e);
                    } else {
                        continue;
                    }
                }
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

pub fn uumain(args: Vec<String>) -> int {
    let program = args[0].clone();
    let opts = vec!(
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    );

    let matches = match getopts::getopts(args.tail(), opts.as_slice()) {
        Ok(m) => m,
        Err(f) => {
            println!("invalid options\n{}", f.to_err_msg());
            return 1;
        }
    };

    if matches.opt_present("help") {
        println!("dd 1.0.0");
        println!("");
        println!("Usage:");
        println!("  {0:s} [OPTIONS]...", program);
        println!("");
        print(getopts::usage(" [FILE].", opts.as_slice()).as_slice());
        return 0;
    }

    if matches.opt_present("version") {
        println!("dd 1.0.0");
        return 0;
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
            let th = bytes::to_human(t);
            let ps = bytes::to_human((t as f64 / s) as u64);

            println!("{}+0 records in", t/action.buffer_size);
            println!("{}+0 records out", t/action.buffer_size);
            println!("{} ({}) bytes copied, {} s, {}/s", t, th, s, ps);
        },
        Err(e) => {
            println!("I/O error: {}", e.desc);
            return 1;
        }
    };

    return 0;
}
