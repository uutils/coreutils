#![crate_name = "shred"]
#![feature(collections, core, io, libc, os, path, rand)]

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
const BLOCK_SIZE: usize = 512;
const NAMESET: &'static str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_.";
// Patterns as shown in the GNU coreutils shred implementation
const PATTERNS: [&'static [u8]; 37] = [
    b"\x00", b"\xFF",
    b"\x55", b"\xAA",
    b"\x24\x92\x49", b"\x49\x24\x92", b"\x6D\xB6\xDB", b"\x92\x49\x24",
        b"\xB6\xDB\x6D", b"\xDB\x6D\xB6",
    b"\x11", b"\x22", b"\x33", b"\x44", b"\x66", b"\x77", b"\x88", b"\x99", b"\xBB", b"\xCC",
        b"\xDD", b"\xEE",
    b"\x10\x00", b"\x12\x49", b"\x14\x92", b"\x16\xDB", b"\x19\x24",
        b"\x1B\x6D", b"\x1D\xB6", b"\x1F\xFF", b"\x18\x88", b"\x19\x99",
        b"\x1A\xAA", b"\x1B\xBB", b"\x1C\xCC", b"\x1D\xDD", b"\x1E\xEE",
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
    gen_type: PassType<'a>,
    rng: Option<RefCell<ThreadRng>>,
}

impl<'a> BytesGenerator<'a> {
    fn new(total_bytes: u64, gen_type: PassType<'a>) -> BytesGenerator {
        let rng = match gen_type {
            PassType::Random => Some(RefCell::new(rand::thread_rng())),
            _ => None,
        };
        
        let gen = BytesGenerator{total_bytes: total_bytes,
                                 bytes_generated: Cell::new(0u64),
                                 block_size: BLOCK_SIZE,
                                 gen_type: gen_type,
                                 rng: rng};
        gen
    }
}

impl<'a> Iterator for BytesGenerator<'a> {
    type Item = Box<[u8]>;
    
    fn next(&mut self) -> Option<Box<[u8]>> {
        if self.bytes_generated.get() == self.total_bytes {
            return None;
        }
        
        let this_block_size = {
            let bytes_left = self.total_bytes - self.bytes_generated.get();
            if bytes_left > self.block_size as u64 { self.block_size }
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

pub fn main() {
    let args = os::args();
    if args.len() == 1 {
        exit!(1);
    }
    let prog_name : String = format!("{}", Path::new(args[0].as_slice()).filename_display());
    let filename = args[1].as_slice();
    wipe_file(filename, 10, prog_name.as_slice(), true);
    return;
}

/* For debugging purposes
fn wait_enter() {
    old_io::stdin().read_line();
}
*/

fn bytes_to_string(bytes: &[u8]) -> String {
    let mut s = String::new();
    while s.len() < 6 {
        for byte in bytes.iter() {
            s.push_str(format!("{:02x}", *byte).as_slice());
        }
    }
    return s;
}

fn wipe_file(path_str: &str, n_passes: usize, prog_name: &str, verbose: bool) {

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
    
    if n_passes <= 3 {
        for _ in 0..3 { pass_sequence.push(PassType::Random) }
    }
    // This filling process is intentionally similar to that used in GNU's implementation
    else {
        let n_patterns = n_passes - 3; // We do three random passes no matter what
        let n_full_arrays = n_patterns / PATTERNS.len(); // How many times can we go through all the patterns?
        let remainder = n_patterns % PATTERNS.len(); // How many do we get through on our last time through?
        for _ in 0..n_full_arrays {
            for p in PATTERNS.iter() {
                pass_sequence.push(PassType::Pattern(*p));
            }
        }
        for i in 0..remainder {
            pass_sequence.push(PassType::Pattern(PATTERNS[i]));
        }
        rand::thread_rng().shuffle(pass_sequence.as_mut_slice()); // randomize the order of application
        pass_sequence.insert(0, PassType::Random); // Insert front
        pass_sequence.push(PassType::Random); // Insert back
        let middle = pass_sequence.len() / 2;
        pass_sequence.insert(middle, PassType::Random); // Insert middle
    }

    for (i, pass_type) in pass_sequence.iter().enumerate() {
        if verbose {
            print!("{}: {}: pass {:2.0}/{:2.0} ", prog_name, path.filename_display(), i+1, n_passes);
            match *pass_type {
                PassType::Random => println!("(random)"),
                PassType::Pattern(p) => println!("({})", bytes_to_string(p)),
            };
        }
        do_pass(&mut file, *pass_type, prog_name); // Ignore failed writes; just keep trying
        file.fsync(); // Sync data & metadata to disk after each pass just in case
        file.seek(0, old_io::SeekStyle::SeekSet);
    }
    let renamed_path: Option<Path> = wipe_name(&path, prog_name, true);
    match renamed_path {
        Some(rp) => { remove_file(&rp, path.filename_str().unwrap_or(""), prog_name, verbose); }
        None => (),
    }
}

fn do_pass(file: &mut fs::File, generator_type: PassType, prog_name: &str) -> Result<(), ()> {
    let mut file_size: u64;

    match file.stat() {
        Ok(stat) => file_size = stat.size,
        Err(e) => {
                eprintln!("{}: {}: Couldn't stat file: {}", prog_name,
                                                            file.path().filename_display(),
                                                            e.desc);
                return Err(());
            }
    };
    
    let mut generator = BytesGenerator::new(file_size, generator_type);
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
                    last_path = new_path.clone();
                    prev_path = new_path;
                    break;
                }
                Err(e) => {
                    eprintln!("{}: {}: COULD NOT RENAME TO {}: {}", prog_name,
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
            eprintln!("{}: {}: COULD NOT REMOVE: {}", prog_name, path.filename_display(), e.desc);
            Err(())
        }
    }
}
