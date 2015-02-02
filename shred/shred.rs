#![crate_name = "shred"]
#![allow(unstable)]

#[macro_use] extern crate log;

extern crate getopts;
extern crate libc;

use std::cell::{Cell, RefCell};
use std::old_io::fs;
use std::old_io::fs::PathExtensions;
use std::old_io;
use std::result::Result;
use std::os;
use std::rand;
use std::rand::{ThreadRng, Rng};
use std::vec::Vec;

extern crate core;
use self::core::fmt;
use self::core::ops::DerefMut;

static NAME: &'static str = "shred";
const BLOCK_SIZE: usize = 512;

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

struct BytesGenerator<'a> {
    total_bytes: u64,
    bytes_generated: Cell<u64>,
    block_size: usize,
    gen_type: PassType<'a>,
    rng: Option<RefCell<ThreadRng>>,
}

impl<'a> BytesGenerator<'a> {
    fn new(total_bytes: u64, gen_type: PassType<'a>) -> BytesGenerator {
        let mut rng = match gen_type {
            PassType::Random => Some(RefCell::new(rand::thread_rng())),
            _ => None,
        };
        
        let gen = BytesGenerator{total_bytes: total_bytes,
                                 bytes_generated: Cell::new(0u64),
                                 block_size: BLOCK_SIZE,
                                 gen_type: gen_type,
                                 rng: rng};
        return gen;
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
                let mut skip = {
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
        return Some(bytes.into_boxed_slice());
    }
}

pub fn main() {
    let args = os::args();
    if args.len() == 1 {
        return;
    }
    let filename = args[1].as_slice();
    
    wipe_file(filename, 10);
    return;
}

/*impl<'a, T: fmt::Display> fmt::Display for AsSlice<T>+'a {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for elem in self.iter() {
            f.write_string(format!("{} ", elem));
        }
        return Ok(());
    }
}*/

fn print_slice<T: fmt::Display>(slice: &[T]) {
    for elem in slice.iter() {
        print!("{} ", elem);
    }
}

fn bytes_to_string(bytes: &[u8]) -> String {
    let mut s = String::new();
    while s.len() < 6 {
        for byte in bytes.iter() {
            s.push_str(format!("{:02x}", *byte).as_slice());
        }
    }
    return s;
}

fn wipe_file(filename: &str, n_passes: usize) {

    let mut pass_sequence: Vec<PassType> = Vec::new();
    
    if n_passes <= 3 {
        for i in 0..3 { pass_sequence.push(PassType::Random) }
    }
    // This filling process is intentionally similar to that used in GNU's implementation
    else {
        let n_patterns = n_passes - 3; // We do three random passes no matter what
        let n_full_arrays = n_patterns / PATTERNS.len(); // How many times can we go through all the patterns?
        let remainder = n_patterns % PATTERNS.len(); // How many do we get through on our last time through?
        for i in 0..n_full_arrays {
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

    let path = Path::new(filename);
    if !path.exists() { panic!("Error: File does not exist"); }
    if !path.is_file() { panic!("Error: Only files may be given as arguments") }
    
    for (i, pass_type) in pass_sequence.iter().enumerate() {
        print!("{:2.0}/{:2.0}: ", i+1, n_passes);
        match *pass_type {
            PassType::Random => println!("(random)"),
            PassType::Pattern(p) => println!("({})", bytes_to_string(p)),
        };
        do_pass(&path, *pass_type);
    }
}

fn do_pass(path: &Path, generator_type: PassType) {
    
    let mut file: fs::File;
    let mut file_size: u64;
    
    match fs::File::open_mode(path, old_io::Open, old_io::Write) {
        Ok(f) => file = f,
        Err(e) => panic!("Error: Could not open file: {}", e),
    };
    match file.stat() {
        Ok(stat) => file_size = stat.size,
        Err(e) => panic!("Error: could not read file stats: {}", e),
    };
    
    let mut generator = BytesGenerator::new(file_size, generator_type);
    for block in generator {
        match file.write(&*block) {
            Ok(_) => (),
            Err(e) => panic!("Write failed! {}", e),
        }
    }
    info!("Pass complete");
}

fn remove_file(path: &Path, verbose: bool) -> Result<(), ()> {
    match fs::unlink(path) {
        Ok(_) => if verbose { println!("Removed '{}'", "<SOME FILE>"); },
        Err(f) => {
            println!("{}", f.to_string());
            return Err(());
        }
    }
    Ok(())
}
