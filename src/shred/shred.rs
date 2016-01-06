#![crate_name = "uu_shred"]

/*
* This file is part of the uutils coreutils package.
*
* (c) Michael Rosenberg <42micro@gmail.com>
* (c) Fort <forticulous@gmail.com>
*
* For the full copyright and license information, please view the LICENSE
* file that was distributed with this source code.
*/

extern crate getopts;
extern crate rand;

use rand::{ThreadRng, Rng};
use std::cell::{Cell, RefCell};
use std::fs;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::SeekFrom;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

#[macro_use]
extern crate uucore;

static NAME: &'static str = "shred";
static VERSION_STR: &'static str = "1.0.0";
const BLOCK_SIZE: usize = 512;
const NAMESET: &'static str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_.";

// Patterns as shown in the GNU coreutils shred implementation
const PATTERNS: [&'static [u8]; 22] = [
    b"\x00",         b"\xFF",         b"\x55",         b"\xAA",
    b"\x24\x92\x49", b"\x49\x24\x92", b"\x6D\xB6\xDB", b"\x92\x49\x24",
    b"\xB6\xDB\x6D", b"\xDB\x6D\xB6", b"\x11",         b"\x22",
    b"\x33",         b"\x44",         b"\x66",         b"\x77",
    b"\x88",         b"\x99",         b"\xBB",         b"\xCC",
    b"\xDD",         b"\xEE"
];

#[derive(Clone, Copy)]
enum PassType<'a> {
    Pattern(&'a [u8]),
    Random
}

// Used to generate all possible filenames of a certain length using NAMESET as an alphabet
struct FilenameGenerator {
    name_len: usize,
    nameset_indices: RefCell<Vec<usize>>, // Store the indices of the letters of our filename in NAMESET
    exhausted: Cell<bool>
}

impl FilenameGenerator {
    fn new(name_len: usize) -> FilenameGenerator {
        let mut indices: Vec<usize> = Vec::new();
        for _ in 0..name_len {
            indices.push(0);
        }
        FilenameGenerator {
            name_len: name_len,
            nameset_indices: RefCell::new(indices),
            exhausted: Cell::new(false)
        }
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
            let c: char = NAMESET.chars().nth(*i).unwrap();
            ret.push(c);
        }
        
        if nameset_indices[0] == NAMESET.len() - 1 {
            self.exhausted.set(true)
        }
        // Now increment the least significant index
        for i in (0..self.name_len).rev() {
            if nameset_indices[i] == NAMESET.len() - 1 {
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
    rng: Option<RefCell<ThreadRng>>
}

impl<'a> BytesGenerator<'a> {
    fn new(total_bytes: u64, gen_type: PassType<'a>, exact: bool) -> BytesGenerator {
        let rng = match gen_type {
            PassType::Random => Some(RefCell::new(rand::thread_rng())),
            _ => None,
        };
        
        BytesGenerator {
            total_bytes: total_bytes,
            bytes_generated: Cell::new(0u64),
            block_size: BLOCK_SIZE,
            exact: exact,
            gen_type: gen_type,
            rng: rng
        }
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
        
        let this_block_size: usize = {
            if !self.exact {
                self.block_size
            } else {
                let bytes_left: u64 = self.total_bytes - self.bytes_generated.get();
                if bytes_left >= self.block_size as u64 {
                    self.block_size
                } else {
                    (bytes_left % self.block_size as u64) as usize
                }
            }
        };
        
        let mut bytes : Vec<u8> = Vec::with_capacity(this_block_size);
        
        match self.gen_type {
            PassType::Random => {
                // This is ok because the vector was
                // allocated with the same capacity
                unsafe {
                    bytes.set_len(this_block_size);
                }
                let mut rng = self.rng.as_ref().unwrap().borrow_mut();
                rng.fill_bytes(&mut bytes[..]);
            }
            PassType::Pattern(pattern) => {
                let skip = {
                    if self.bytes_generated.get() == 0 {
                      0
                    } else {
                      (pattern.len() as u64 % self.bytes_generated.get()) as usize
                    }
                };
                // Same range as 0..this_block_size but we start with the right index
                for i in skip..this_block_size + skip {
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

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    // TODO: Add force option
    opts.optopt("n", "iterations", "overwrite N times instead of the default (3)", "N");
    opts.optopt("s", "size", "shred this many bytes (suffixes like K, M, G accepted)", "FILESIZE");
    opts.optflag("u", "remove", "truncate and remove the file after overwriting; See below");
    opts.optflag("v", "verbose", "show progress");
    opts.optflag("x", "exact", "do not round file sizes up to the next full block; \
                                this is the default for non-regular files");
    opts.optflag("z", "zero", "add a final overwrite with zeros to hide shredding");
    opts.optflag("", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m)  => m,
        Err(e) => panic!("Invalid options\n{}", e)
    };

    if matches.opt_present("help") {
        show_help(&opts);
        return 0;
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION_STR);
        return 0;
    } else if matches.free.is_empty() {
        println!("{}: Missing an argument", NAME);
        println!("For help, try '{} --help'", NAME);
        return 0;
    } else {
        let iterations = match matches.opt_str("iterations") {
            Some(s) => match s.parse::<usize>() {
                           Ok(u) => u,
                           Err(_) => {
                               println!("{}: Invalid number of passes", NAME);
                               return 1;
                           }
                       },
            None => 3
        };
        let remove = matches.opt_present("remove");
        let size = get_size(matches.opt_str("size"));
        let exact = matches.opt_present("exact") && size.is_none(); // if -s is given, ignore -x
        let zero = matches.opt_present("zero");
        let verbose = matches.opt_present("verbose");
        for path_str in matches.free.into_iter() {
            wipe_file(&path_str, iterations, remove,
                      size, exact, zero, verbose);
        }
    }

    0
}

fn show_help(opts: &getopts::Options) {
    println!("Usage: {} [OPTION]... FILE...", NAME);
    println!("Overwrite the specified FILE(s) repeatedly, in order to make it harder \
              for even very expensive hardware probing to recover the data.");
    println!("{}", opts.usage(""));
    println!("Delete FILE(s) if --remove (-u) is specified.  The default is not to remove");
    println!("the files because it is common to operate on device files like /dev/hda,");
    println!("and those files usually should not be removed.");
    println!("");
    println!("CAUTION: Note that {} relies on a very important assumption:", NAME);
    println!("that the file system overwrites data in place.  This is the traditional");
    println!("way to do things, but many modern file system designs do not satisfy this");
    println!("assumption.  The following are examples of file systems on which {} is", NAME);
    println!("not effective, or is not guaranteed to be effective in all file system modes:");
    println!("");
    println!("* log-structured or journaled file systems, such as those supplied with");
    println!("AIX and Solaris (and JFS, ReiserFS, XFS, Ext3, etc.)");
    println!("");
    println!("* file systems that write redundant data and carry on even if some writes");
    println!("fail, such as RAID-based file systems");
    println!("");
    println!("* file systems that make snapshots, such as Network Appliance's NFS server");
    println!("");
    println!("* file systems that cache in temporary locations, such as NFS");
    println!("version 3 clients");
    println!("");
    println!("* compressed file systems");
    println!("");
    println!("In the case of ext3 file systems, the above disclaimer applies");
    println!("(and {} is thus of limited effectiveness) only in data=journal mode,", NAME);
    println!("which journals file data in addition to just metadata.  In both the");
    println!("data=ordered (default) and data=writeback modes, {} works as usual.", NAME);
    println!("Ext3 journaling modes can be changed by adding the data=something option");
    println!("to the mount options for a particular file system in the /etc/fstab file,");
    println!("as documented in the mount man page (man mount).");
    println!("");
    println!("In addition, file system backups and remote mirrors may contain copies");
    println!("of the file that cannot be removed, and that will allow a shredded file");
    println!("to be recovered later.");
}

// TODO: Add support for all postfixes here up to and including EiB
//       http://www.gnu.org/software/coreutils/manual/coreutils.html#Block-size
fn get_size(size_str_opt: Option<String>) -> Option<u64> {
    if size_str_opt.is_none() {
        return None;
    }
    
    let mut size_str = size_str_opt.as_ref().unwrap().clone();
    // Immutably look at last character of size string
    let unit = match size_str.chars().last().unwrap() {
        'K' => { size_str.pop();  1024u64 }
        'M' => { size_str.pop(); (1024 * 1024) as u64 }
        'G' => { size_str.pop(); (1024 * 1024 * 1024) as u64 }
         _   => { 1u64 }
    };
    
    let coeff = match size_str.parse::<u64>() {
        Ok(u) => u,
        Err(_) => {
            println!("{}: {}: Invalid file size", NAME, size_str_opt.unwrap());
            exit!(1);
        }
    };
    
    Some(coeff * unit)
}

fn pass_name(pass_type: &PassType) -> String {
    match *pass_type {
        PassType::Random => String::from("random"),
        PassType::Pattern(bytes) => {
            let mut s: String = String::new();
            while s.len() < 6 {
                for b in bytes {
                    let readable: String = format!("{:x}", b);
                    s.push_str(&readable);
                }
            }
            s
        }
    }
}

fn wipe_file(path_str: &str, n_passes: usize, remove: bool,
             size: Option<u64>, exact: bool, zero: bool, verbose: bool) {

    // Get these potential errors out of the way first
    let path: &Path = Path::new(path_str);
    if !path.exists() {
        println!("{}: {}: No such file or directory", NAME, path.display()); return;
    }
    if !path.is_file() {
        println!("{}: {}: Not a file", NAME, path.display()); return;
    }

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
            for p in &PATTERNS {
                pass_sequence.push(PassType::Pattern(*p));
            }
        }
        for i in 0..remainder {
            pass_sequence.push(PassType::Pattern(PATTERNS[i]));
        }
        rand::thread_rng().shuffle(&mut pass_sequence[..]); // randomize the order of application
        
        let n_random = 3 + n_passes/10; // Minimum 3 random passes; ratio of 10 after
        // Evenly space random passes; ensures one at the beginning and end
        for i in 0..n_random {
            pass_sequence[i * (n_passes - 1)/(n_random - 1)] = PassType::Random;
        }
    }
    
    // --zero specifies whether we want one final pass of 0x00 on our file
    if zero {
        pass_sequence.push(PassType::Pattern(b"\x00"));
    }

    {
        let total_passes: usize = pass_sequence.len();
        let mut file: File = OpenOptions::new().write(true)
                                               .truncate(false)
                                               .open(path)
                                               .expect("Failed to open file for writing");

        for (i, pass_type) in pass_sequence.iter().enumerate() {
            if verbose {
                let pass_name: String = pass_name(pass_type);
                if total_passes.to_string().len() == 1 {
                    println!("{}: {}: pass {}/{} ({})... ", NAME, path.display(), i + 1, total_passes, pass_name);
                }
                else {
                    println!("{}: {}: pass {:2.0}/{:2.0} ({})... ", NAME, path.display(), i + 1, total_passes, pass_name);
                }
            }
            // size is an optional argument for exactly how many bytes we want to shred
            do_pass(&mut file, path, *pass_type, size, exact).expect("File write pass failed"); // Ignore failed writes; just keep trying
        }
    }

    if remove {
        do_remove(path, path_str, verbose).expect("Failed to remove file");
    }
}

fn do_pass(file: &mut File, path: &Path, generator_type: PassType,
           given_file_size: Option<u64>, exact: bool) -> Result<(), io::Error> {

    try!(file.seek(SeekFrom::Start(0)));

    // Use the given size or the whole file if not specified
    let size: u64 = given_file_size.unwrap_or(try!(get_file_size(path)));

    let generator = BytesGenerator::new(size, generator_type, exact);

    for block in generator {
        try!(file.write_all(&*block));
    }

    try!(file.sync_data());

    Ok(())
}

fn get_file_size(path: &Path) -> Result<u64, io::Error> {
    let size: u64 = try!(fs::metadata(path)).len();

    Ok(size)
}

// Repeatedly renames the file with strings of decreasing length (most likely all 0s)
// Return the path of the file after its last renaming or None if error
fn wipe_name(orig_path: &Path, verbose: bool) -> Option<PathBuf> {
    let file_name_len: usize = orig_path.file_name().unwrap().to_str().unwrap().len();
    
    let mut last_path: PathBuf = PathBuf::from(orig_path);
    
    for length in (1..file_name_len + 1).rev() {
        for name in FilenameGenerator::new(length) {
            let new_path: PathBuf = orig_path.with_file_name(name);
            // We don't want the filename to already exist (don't overwrite)
            // If it does, find another name that doesn't
            if new_path.exists() {
                continue;
            }
            match fs::rename(&last_path, &new_path) {
                Ok(()) => {
                    if verbose {
                        println!("{}: {}: renamed to {}", NAME,
                                                          last_path.display(),
                                                          new_path.display());
                    }
                   
                    // Sync every file rename
                    {
                        let new_file: File = File::open(new_path.clone()).expect("Failed to open renamed file for syncing");
                        new_file.sync_all().expect("Failed to sync renamed file");
                    }

                    last_path = new_path;
                    break;
                }
                Err(e) => {
                    println!("{}: {}: Couldn't rename to {}: {}", NAME,
                                                                  last_path.display(),
                                                                  new_path.display(),
                                                                  e);
                    return None;
                }
            }
        } // If every possible filename already exists, just reduce the length and try again
    }

    Some(last_path)
}

fn do_remove(path: &Path, orig_filename: &str, verbose: bool) -> Result<(), io::Error> {
    if verbose {
        println!("{}: {}: removing", NAME, orig_filename);
    }

    let renamed_path: Option<PathBuf> = wipe_name(&path, verbose);
    match renamed_path {
        Some(rp) => {
            try!(fs::remove_file(rp));
        },
        None => ()
    }

    if verbose {
        println!("{}: {}: removed", NAME, orig_filename);
    }

    Ok(())
}
