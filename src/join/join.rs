#![crate_name = "join"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Piotr Kawałek <p0kawalek@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![allow(dead_code)]

extern crate getopts;
extern crate libc;

use libc::consts::os::posix88::STDIN_FILENO;
use libc::funcs::posix88::unistd::isatty;
use libc::types::os::arch::c95::c_int;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, stdin, Write};
use std::path::Path;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "join";
static VERSION: &'static str = "0.0.1";

static DECIMAL_PT: char = '.';
static THOUSANDS_SEP: char = ',';

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optopt("a", "", "also  print  unpairable lines from file FILENUM, where FILENUM is 1 or 2, corresponding to FILE1 or FILE2 ", "FILENUM");
    opts.optopt("e", "", "replace missing input fields with EMPTY", "EMPTY");
    opts.optflag("i", "ignore-case", "ignore differences in case when comparing fields");
    opts.optopt("j", "", "equivalent to '-1 FIELD -2 FIELD'", "FIELD");
    opts.optopt("o", "", "obey FORMAT while constructing output line", "FORMAT");
    opts.optopt("t", "", "use CHAR as input and output field separator", "CHAR");
    opts.optopt("v", "", "like -a FILENUM, but suppress joined output lines", "FILENUM");
    opts.optopt("1", "", "join on this FIELD of file 1", "FIELD");
    opts.optopt("2", "", "join on this FIELD of file 2", "FIELD");
    opts.optflag("", "check-order", "check that the input is correctly sorted, even if all input lines are pairable");
    opts.optflag("", "nocheck-order", "do not check that the input is correctly sorted");
    opts.optflag("", "header", "treat  the  first  line  in  each file as field headers, print them without trying to pair them");
    opts.optflag("", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "Invalid options\n{}", f)
    };
    if matches.opt_present("help") {
        let msg = format!("{0} {1}

Usage:
 {0} [OPTION]... [FILE]...
For  each  pair of input lines with identical join fields, 
write a line to standard output. The default join field is 
the first, delimited by whitespace.  When FILE1 or FILE2 
(not both) is -, read standard input.", NAME, VERSION);
        print!("{}", opts.usage(&msg));
        return 0;
    }

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    let mut options = JoinOpts{
        first_field: 0,
        second_field: 0,
        write_unpaired: -1,
        case_sensitive: false,
        check_order: false,
        classic_format: true,
        header: false,
        supress_paired: false, 
        white_spc_sep: true,
        separator: ' ', 
    };

    //FILENUM -a && FIELD -j
    options.first_field = match matches.opt_str("j"){
        Some(x) => x.parse().unwrap(),
        None    => 1
    };   
    options.second_field = options.first_field;

    options.first_field = match matches.opt_str("1"){
        Some(x) => x.parse().unwrap(),
        None    => options.first_field
    };          
    
    options.second_field = match matches.opt_str("2"){
        Some(x) => x.parse().unwrap(),
        None    => options.second_field
    }; 

    options.first_field = options.first_field - 1;
    options.second_field = options.second_field - 1;      

    //Separator -t
    let separator: String = match matches.opt_str("t"){
        Some(x) =>  x,
        None    => "".to_string()
    };
    let separator: Vec<char> = separator.chars().collect();
    match separator.len()  {
        0 => (),
        1 => {options.white_spc_sep = false;
              options.separator = separator[0]},
        _ => {panic!("multiletter separator is supressed");}        
    }

    //get file names
    let files = matches.free;
    if files.is_empty() {
        crash!(1, "Type \"join --help\" for more info\n");
    }
    if files.len() != 2 {
        crash!(1, "Wrong number of input files\n");
    }
    if files[0] == "-" && files[1] == "-" {
        crash!(1, "You shouldn't type \"-\" for both files\n");
    }
    
    exec(files, options);
    0
}

struct JoinOpts{
    first_field: usize,
    second_field: usize,
    write_unpaired: i32,
    case_sensitive: bool,
    check_order: bool,
    classic_format: bool,
    header: bool,
    supress_paired: bool,
    white_spc_sep: bool, 
    separator: char, 
}

fn exec(files: Vec<String>, opts: JoinOpts) {
    let (first_reader, _) = match open(&files[0]) {
        Some(x) => x,
        None => panic!("No such file or directory"),
    };
    let (second_reader, _) = match open(&files[1]) {
        Some(x) => x,
        None => panic!("No such file or directory"),
    };
    let mut first_lines = BufReader::new(first_reader).lines();
    let mut second_lines = BufReader::new(second_reader).lines();
    let mut first_opt = first_lines.next();
    let mut second_opt = second_lines.next();
    
    'outer: while first_opt.is_some() && second_opt.is_some() {
        let mut first = first_opt.unwrap().unwrap();
        let mut second = second_opt.unwrap().unwrap();
        while first == "" {
            first_opt = first_lines.next();
            if first_opt.is_none() {
                break 'outer;
            }
            first = first_opt.unwrap().unwrap();
        }
        while second == "" {
            second_opt = second_lines.next();
            if second_opt.is_none() {
                break 'outer;
            }
            second = second_opt.unwrap().unwrap();
        }
        
        let first_splited: Vec<&str>;
        let second_splited: Vec<&str>;
        let space = " ";
        let other = &opts.separator.to_string();
        let sep: &str;
        if opts.white_spc_sep {
            first_splited = first.split_whitespace().collect();
            second_splited = second.split_whitespace().collect();
            sep = space;
        }
        else{
            first_splited = first.split(opts.separator).collect();
            second_splited = second.split(opts.separator).collect();
            sep = other;
        }
        if first_splited.len() > opts.first_field
        && second_splited.len() > opts.second_field
        && first_splited.get(opts.first_field).unwrap()
           == second_splited.get(opts.second_field).unwrap() {
            let mut i=0;
            let mut out_string: String = 
               first_splited.get(opts.first_field)
                    .unwrap().to_string();
            for x in first_splited {
                if i!=opts.first_field {
                    out_string.push_str(sep);
                    out_string.push_str(x);
                }
                i = i+1;
            }
            i=0;
            for x in second_splited {
                if i!= opts.second_field {
                    out_string.push_str(sep);
                    out_string.push_str(x);
                }
                i = i+1
            }
            println!("{}", out_string);
        }

        first_opt = first_lines.next();
        second_opt = second_lines.next();
    }  
}

// from cat.rs
fn open<'a>(path: &str) -> Option<(Box<Read + 'a>, bool)> {
    if path == "-" {
        let stdin = stdin();
        let interactive = unsafe { isatty(STDIN_FILENO) } != 0 as c_int;
        return Some((Box::new(stdin) as Box<Read>, interactive));
    }

    match File::open(Path::new(path)) {
        Ok(f) => Some((Box::new(f) as Box<Read>, false)),
        Err(e) => {
            show_error!("sort: {0}: {1}", path, e.to_string());
            None
        },
    }
}
