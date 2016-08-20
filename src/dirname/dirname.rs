#![crate_name = "uu_dirname"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[macro_use]
extern crate uucore;

use std::path::Path;

static NAME: &'static str = "dirname"; 
static SYNTAX: &'static str = "[OPTION] NAME..."; 
static SUMMARY: &'static str = "strip last component from file name"; 
static LONG_HELP: &'static str = "
 Output each NAME with its last non-slash component and trailing slashes
 removed; if NAME contains no /'s, output '.' (meaning the current
 directory).
"; 

pub fn uumain(args: Vec<String>) -> i32 {
    let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
        .optflag("z", "zero", "separate output with NUL rather than newline")
        .parse(args);

    let separator = if matches.opt_present("zero") {"\0"} else {"\n"};

    if !matches.free.is_empty() {
        for path in &matches.free {
            let p = Path::new(path);
            match p.parent() {
                Some(d) => {
                    if d.components().next() == None {
                        print!(".")
                    } else {
                        print!("{}", d.to_string_lossy());
                    }
                }
                None => {
                    if p.is_absolute() || path == "/" {
                        print!("/");
                    } else {
                        print!(".");
                    }
                }
            }
            print!("{}", separator);
        }
    } else {
        println!("{0}: missing operand", NAME);
        println!("Try '{0} --help' for more information.", NAME);
        return 1;
    }

    0
}
