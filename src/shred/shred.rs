#![crate_name = "shred"]
#![feature(collections, core, io, libc, os, path, rand)]
/*
* This file is part of the uutils coreutils package.
*
* (c) Michael Rosenberg <42micro@gmail.com>
*
* For the full copyright and license information, please view the LICENSE
* file that was distributed with this source code.
*/

extern crate getopts;

use std::cell::{Cell, RefCell};
use std::old_io::fs;
use std::old_io::fs::PathExtensions;
use std::old_io;
use std::os;
use std::result::Result;
use std::rand;
use std::rand::{ThreadRng, Rng};

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "shred";
static VERSION_STR: &'static str = "1.0.0";
const BLOCK_SIZE: usize = 512;
const NAMESET: &'static str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_.";

// Patterns as shown in the GNU coreutils shred implementation
const PATTERNS: [&'static [u8]; 22] = [
    b"\x00", b"\xFF",
    b"\x55", b"\xAA",
    b"\x24\x92\x49", b"\x49\x24\x92", b"\x6D\xB6\xDB", b"\x92\x49\x24",
        b"\xB6\xDB\x6D", b"\xDB\x6D\xB6",
    b"\x11", b"\x22", b"\x33", b"\x44", b"\x66", b"\x77", b"\x88", b"\x99", b"\xBB", b"\xCC",
        b"\xDD", b"\xEE",
];

#[derive(Copy)]
enum PassType<'a> {
    Pattern(&'a [u8]),
    Random,
}

// Used to generate all possible filenames of a certain length using NAMESET as an alphabet
struct FilenameGenerator {
    name_len: usize,
    nameset_indices: RefCell<Vec<usize>>, // Store the indices of the letters of our filename in NAMESET
    exhausted: Cell<bool>,
}

impl FilenameGenerator {
    fn new(name_len: usize) -> FilenameGenerator {
        let mut indices = Vec::new();
        for _ in 0..name_len {
            indices.push(0);
        }
        FilenameGenerator{name_len: name_len,
                          nameset_indices: RefCell::new(indices),
                          exhausted: Cell::new(false)}
    }
}

impl Iterator for FilenameGenerator {
    type Item = String;
    
    fn next(&mut self) -> Option<String> {
        if self.exhausted.get() {
            return None;
        }
        
        let mut nameset_indices = self.nameset_indices.borrow_mut();
        
        // Make the return value, then increment
        let mut ret = String::new();
        for i in nameset_indices.iter() {
            ret.push(NAMESET.char_at(*i));
        }
        
        if nameset_indices[0] == NAMESET.len()-1 { self.exhausted.set(true) }
        // Now increment the least significant index
        for i in range(0, self.name_len).rev() {
            if nameset_indices[i] == NAMESET.len()-1 {
                nameset_indices[i] = 0; // Carry the 1
                continue;
            }
            else {
                nameset_indices[i] += 1;
                break;
            }
        }
        
        Some(ret)
    }
}

// Used to generate blocks of bytes of size <= BLOCK_SIZE based on either a give pattern
// or randomness
struct BytesGenerator<'a> {
    total_bytes: u64,
    bytes_generated: Cell<u64>,
    block_size: usize,
    exact: bool, // if false, every block's size is block_size
    gen_type: PassType<'a>,
    rng: Option<RefCell<ThreadRng>>,
}

impl<'a> BytesGenerator<'a> {
    fn new(total_bytes: u64, gen_type: PassType<'a>, exact: bool) -> BytesGenerator {
        let rng = match gen_type {
            PassType::Random => Some(RefCell::new(rand::thread_rng())),
            _ => None,
        };
        
        let gen = BytesGenerator{total_bytes: total_bytes,
                                 bytes_generated: Cell::new(0u64),
                                 block_size: BLOCK_SIZE,
                                 exact: exact,
                                 gen_type: gen_type,
                                 rng: rng};
        gen
    }
}

impl<'a> Iterator for BytesGenerator<'a> {
    type Item = Box<[u8]>;
    
    fn next(&mut self) -> Option<Box<[u8]>> {
        // We go over the total_bytes limit when !self.exact and total_bytes isn't a multiple
        // of self.block_size
        if self.bytes_generated.get() >= self.total_bytes {
            return None;
        }
        
        let this_block_size = {
            let bytes_left = self.total_bytes - self.bytes_generated.get();
            if !self.exact { self.block_size }
            else if bytes_left >= self.block_size as u64 { self.block_size }
            else { (bytes_left % self.block_size as u64) as usize }
        };
        
        let mut bytes : Vec<u8> = Vec::with_capacity(this_block_size);
        
        match self.gen_type {
            PassType::Random => {
                let mut rng = self.rng.as_ref().unwrap().borrow_mut();
                unsafe {
                    bytes.set_len(this_block_size);
                    rng.fill_bytes(bytes.as_mut_slice());
                }
            }
            PassType::Pattern(pattern) => {
                let skip = {
                    if self.bytes_generated.get() == 0 { 0 }
                    else { (pattern.len() as u64 % self.bytes_generated.get()) as usize }
                };
                // Same range as 0..this_block_size but we start with the right index
                for i in skip..this_block_size+skip {
                    let index = i % pattern.len();
                    bytes.push(pattern[index]);
                }
            }
        };
        
        let new_bytes_generated = self.bytes_generated.get() + this_block_size as u64;
        self.bytes_generated.set(new_bytes_generated);
        Some(bytes.into_boxed_slice())
    }
}

// TODO: Add support for all postfixes here up to and including EiB
//       http://www.gnu.org/software/coreutils/manual/coreutils.html#Block-size
fn get_size(size_str_opt: Option<String>, prog_name: &str) -> Option<u64> {
    if size_str_opt.is_none() { return None; }
    
    let mut size_str = size_str_opt.as_ref().unwrap().clone();
    // Immutably look at last character of size string
    let unit = match size_str.as_slice().char_at_reverse(size_str.len()) {
        'K' => { size_str.pop();  1024u64 }
        'M' => { size_str.pop(); (1024*1024) as u64 }
        'G' => { size_str.pop(); (1024*1024*1024) as u64 }
         _   => { 1u64 }
    };
    
    let coeff = match size_str.parse::<u64>() {
        Ok(u) => u,
        Err(_) => {
            eprintln!("{}: {}: Invalid file size", prog_name, size_str_opt.unwrap());
            exit!(1);
        }
    };
    
    Some(coeff*unit)
}

pub fn uumain(args: Vec<String>) -> isize {
    let prog_name: String = format!("{}", Path::new(args[0].as_slice()).filename_display());

    let opts = [
        getopts::optopt("n", "iterations", "overwrite N times instead of the default (3)", "N"),
        getopts::optopt("s", "size", "shred this many bytes (suffixes like K, M, G accepted)", "FILESIZE"),
        getopts::optflag("u", "remove", "truncate and remove the file after overwriting; See below"),
        getopts::optflag("v", "verbose", "show progress"),
        getopts::optflag("x", "exact", "do not round file sizes up to the next full block;\
                                        this is the default for non-regular files"),
        getopts::optflag("z", "zero", "add a final overwrite with zeros to hide shredding"),
        getopts::optflag("", "help", "display this help and exit"),
        getopts::optflag("", "version", "output version information and exit"),
    ];
    
    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => {
            eprintln!("{}: {}", prog_name, f);
            exit!(1);
        }
    };

    if matches.opt_present("help") {
        println!("Usage: {} [OPTION]... FILE...", prog_name);
        println!("Overwrite the specified FILE(s) repeatedly, in order to make it harder");
        print!("for even very expensive hardware probing to recover the data.");
        println!("{}", getopts::usage("", &opts));
        println!("Delete FILE(s) if --remove (-u) is specified.  The default is not to remove");
        println!("the files because it is common to operate on device files like /dev/hda,");
        println!("and those files usually should not be removed.");
        println!("");
        exit!(0);
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION_STR);
        exit!(0);
    } else if matches.free.is_empty() {
        eprintln!("{}: Missing an argument", NAME);
        eprintln!("For help, try '{0} --help'", prog_name);
        exit!(0);
    } else {
        let iterations = match matches.opt_str("iterations") {
            Some(s) => match s.parse::<usize>() {
                           Ok(u) => u,
                           Err(_) => {
                               eprintln!("{}: Invalid number of passes", prog_name);
                               exit!(1);
                           }
                       },
            None => 3us
        };
        let remove = matches.opt_present("remove");
        let size = get_size(matches.opt_str("size"), prog_name.as_slice());
        let exact = matches.opt_present("exact") && size.is_none(); // if -s is given, ignore -x
        let zero = matches.opt_present("zero");
        let verbose = matches.opt_present("verbose");
        for path_str in matches.free.into_iter() {
            wipe_file(path_str.as_slice(), iterations, prog_name.as_slice(),
                      remove, size, exact, zero, verbose);
        }
    }
    
    exit!(0);
}

// For debugging purposes
/*
fn wait_enter() {
    old_io::stdin().read_line();
}
*/

fn bytes_to_string(bytes: &[u8]) -> String {
    let mut s = String::new();
    while s.len() < 6 {
        for byte in bytes.iter() {
            s.push_str(format!("{:02x}", *byte).as_slice());
            if s.len() == 6 { break; }
        }
    }
    return s;
}

fn wipe_file(path_str: &str, n_passes: usize, prog_name: &str,
             remove: bool, size: Option<u64>, exact: bool, zero: bool, verbose: bool) {

    // Get these potential errors out of the way first
    let path = Path::new(path_str);
    if !path.exists() { eprintln!("{}: {}: No such file or directory", prog_name, path.display()); return; }
    if !path.is_file() { eprintln!("{}: {}: Not a file", prog_name, path.display()); return; }
    
    let mut file = match fs::File::open_mode(&path, old_io::Open, old_io::Write) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}: {}: Couldn't open file for writing: {}", prog_name,
                                                                    path.filename_display(), e.desc);
            return;
        }
    };

    // Fill up our pass sequence
    
    let mut pass_sequence: Vec<PassType> = Vec::new();
    
    if n_passes <= 3 { // Only random passes if n_passes <= 3
        for _ in 0..n_passes { pass_sequence.push(PassType::Random) }
    }
    // First fill it with Patterns, shuffle it, then evenly distribute Random
    else {
        let n_full_arrays = n_passes / PATTERNS.len(); // How many times can we go through all the patterns?
        let remainder = n_passes % PATTERNS.len(); // How many do we get through on our last time through?
        
        for _ in 0..n_full_arrays {
            for p in PATTERNS.iter() {
                pass_sequence.push(PassType::Pattern(*p));
            }
        }
        for i in 0..remainder {
            pass_sequence.push(PassType::Pattern(PATTERNS[i]));
        }
        rand::thread_rng().shuffle(pass_sequence.as_mut_slice()); // randomize the order of application
        
        let n_random = 3us + n_passes/10us; // Minimum 3 random passes; ratio of 10 after
        // Evenly space random passes; ensures one at the beginning and end
        for i in 0us..n_random {
            pass_sequence[i * (n_passes - 1us)/(n_random - 1us)] = PassType::Random;
        }
    }
    
    // --zero specifies whether we want one final pass of 0x00 on our file
    if zero { pass_sequence.push(PassType::Pattern(b"\x00")); }
    let total_passes = n_passes + { if zero { 1 } else { 0 } };

    for (i, pass_type) in pass_sequence.iter().enumerate() {
        if verbose {
            if total_passes.to_string().len() == 1 {
                print!("{}: {}: pass {}/{} ", prog_name, path.filename_display(), i+1, total_passes);
            }
            else {
                print!("{}: {}: pass {:2.0}/{:2.0} ", prog_name, path.filename_display(), i+1, total_passes);
            }
            match *pass_type {
                PassType::Random => println!("(random)"),
                PassType::Pattern(p) => println!("({})", bytes_to_string(p)),
            };
        }
        // size is an optional argument for exactly how many bytes we want to shred
        do_pass(&mut file, *pass_type, size, exact, prog_name); // Ignore failed writes; just keep trying
        file.fsync(); // Sync data & metadata to disk after each pass just in case
        file.seek(0, old_io::SeekStyle::SeekSet);
    }
    
    if remove {
        let renamed_path: Option<Path> = wipe_name(&path, prog_name, true);
        match renamed_path {
            Some(rp) => { remove_file(&rp, path.filename_str().unwrap_or(""), prog_name, verbose); }
            None => (),
        }
    }
}

fn do_pass(file: &mut fs::File, generator_type: PassType,
           given_file_size: Option<u64>, exact: bool, prog_name: &str) -> Result<(), ()> {
           
    let real_file_size = match file.stat() {
        Ok(stat) => stat.size,
        Err(e) => {
                eprintln!("{}: {}: Couldn't stat file: {}", prog_name,
                                                            file.path().filename_display(),
                                                            e.desc);
                return Err(());
            }
    };
    
    // Recall --size specifies how many bytes we want to shred
    let mut generator = match given_file_size {
        Some(given) => BytesGenerator::new(given, generator_type, exact),
        None        => BytesGenerator::new(real_file_size, generator_type, exact),
    };
    
    for block in generator {
        match file.write_all(&*block) {
            Ok(_) => (),
            Err(e) => {
                eprintln!("{}: {}: Couldn't write to file: {}", prog_name,
                                                                file.path().filename_display(),
                                                                e.desc);
                return Err(());
            }
        }
    }
    return Ok(());
}

// Repeatedly renames the file with strings of decreasing length (most likely all 0s)
// Return the path of the file after its last renaming or None if error
fn wipe_name(file_path: &Path, prog_name: &str, verbose: bool) -> Option<Path> {
    let basename_len: usize = format!("{}", file_path.filename_display()).len();
    let mut prev_path = file_path.clone();
    let dir_path: Path = file_path.dir_path();
    
    // make a fs::File for the containing directory so we can call fsync() after every rename
    let mut dir_file = match fs::File::open_mode(&dir_path, old_io::Open, old_io::Read) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}: {}: Couldn't open directory as read-only", prog_name,
                                                                      dir_path.display());
            return None;
        }
    };
    
    let mut last_path: Path = Path::new(""); // for use inside the loop
    
    for length in range(1, basename_len+1).rev() {
        for name in FilenameGenerator::new(length) {
            let new_path = dir_path.join(name.as_slice());
            match fs::stat(&new_path) {
                Err(_) => (), // Good. We don't want the filename to already exist (don't overwrite)
                Ok(_) => continue, // If it does, find another name that doesn't
            }
            match fs::rename(&prev_path, &new_path) {
                Ok(()) => {
                    if verbose {
                        println!("{}: {}: renamed to {}", prog_name,
                                                          prev_path.filename_display(),
                                                          new_path.filename_display());
                    }
                    // Sync this change to disk immediately; Note: this is equivalent to the
                    // --remove=wipesync option in coreutils' shred. Here, it is the only option
                    dir_file.fsync();
                    
                    last_path = new_path.clone();
                    prev_path = new_path;
                    break;
                }
                Err(e) => {
                    eprintln!("{}: {}: Couldn't rename to {}: {}", prog_name,
                                                                   prev_path.filename_display(),
                                                                   new_path.filename_display(),
                                                                   e.desc);
                    return None;
                }
            }
        } // If every possible filename already exists, just reduce the length and try again
    }
    return Some(last_path);
}

fn remove_file(path: &Path, orig_filename: &str, prog_name: &str, verbose: bool) -> Result<(), ()> {
    match fs::unlink(path) {
        Ok(_) => {
            if verbose { println!("{}: {}: removed", prog_name, orig_filename); }
            Ok(())
        }
        Err(e) => {
            eprintln!("{}: {}: Couldn't remove {}", prog_name, path.filename_display(), e.desc);
            Err(())
        }
    }
}
