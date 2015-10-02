#![crate_name = "stat"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Carlos Liam <carlos@aarzee.me>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: stat (GNU coreutils) 8.XX */

extern crate getopts;
extern crate libc;
extern crate regex;
extern crate pgs_files;

use getopts::Options;
use std::i16;
use std::collections::HashMap;
use std::fs::{metadata, Metadata};
use std::os::unix::fs::MetadataExt;
use regex::{Regex, Captures};
use pgs_files::passwd::get_entry_by_uid;
use pgs_files::group::get_entry_by_gid;

use self::FileType::*;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "stat";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();
    opts.optflag("L", "dereference", "follow links");
    opts.optflag("f", "file-system", "display file system status instead of file status");
    opts.optopt("c", "format", "use the specified FORMAT instead of the default", "FORMAT");
    opts.optopt("", "printf", "like --format, but interpret backslash escapes and no mandatory newline", "FORMAT");
    opts.optflag("t", "terse", "print the information in terse form");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!("Invalid options\n{}", f),
    };
    if matches.opt_present("help") {
        let msg = format!("{} {}\n\n\
        Usage:\n  {0} [OPTION]... [FILE]...\n\n\
        Display file or file system status.",
                          NAME,
                          VERSION);

        print!("{}", opts.usage(&msg));
        return 0;
    }
    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }
    let format =
        if let Some(f) = matches.opt_str("printf") {
            f
        }
        else if let Some(mut f) = matches.opt_str("format") {
            f.push('\n');
            f
        }
        else {
            if matches.opt_present("terse") {
                "%n %s %b %f %u %g %D %i %h %t %T %X %Y %Z %W %o\n".to_string()
            }
            else {
                "  File: %N\n  Size: %-10s\tBlocks: %-10b IO Block: %-6o %F\n\
                 Access: (%04a/%10.10A)  Uid: (%5u/%8U)   Gid: (%5g/%8G)\n".to_string()
            }
        };
    let files = matches.free;

    for file in files {
        match metadata(&file) {
            Ok(m) => {
                let mut map = HashMap::new();
                let mode = m.mode() as u16;
                map.insert("n", format!("{}", &file));
                map.insert("N", format!("‘{}’", &file));
                map.insert("a", format!("{:o}", mode as u16));
                map.insert("b", format!("{}", m.blocks()));
                map.insert("o", format!("{}", m.blksize()));
                map.insert("s", format!("{}", m.size()));
                map.insert("u", format!("{}", m.uid()));
                map.insert("U", get_entry_by_uid(m.uid()).unwrap().name);
                map.insert("g", format!("{}", m.gid()));
                map.insert("G", get_entry_by_gid(m.gid()).unwrap().name);
                let filetype = filetype(m);
                map.insert("A", modestr(mode, filetype));
                map.insert("F", filetypestring(filetype));
                print!("{}", printfstyle(&format, map));
            }
            Err(f) => println!("stat: cannot stat: {}", f)
        }
    }


    0
}

#[derive(Clone, Copy)]
enum FileType {
    Directory,
    File,
    Unknown
}

fn filetype(metadata: Metadata) -> FileType {
    if metadata.is_dir() {
        Directory
    }
    else if metadata.is_file() {
        File
    }
    else {
        Unknown
    }
}

fn filetypestring(filetype: FileType) -> String {
    match filetype {
        Directory => "directory",
        File => "regular file",
        Unknown => ""
    }.to_owned()
}

fn modestr(mode: u16, filetype: FileType) -> String {
    format!("{}{}{}{}",
            match filetype {
                Directory => 'd',
                File => '-',
                Unknown => '?'
            },
            rwx(((mode & 0o700) / 0o100) as u8),
            rwx(((mode & 0o070) / 0o010) as u8),
            rwx(((mode & 0o007) / 0o001) as u8)
    )
}

fn rwx(digit: u8) -> String {
    format!("{}{}{}",
        if digit & 0b100 != 0b000 { 'r' } else { '-' },
        if digit & 0b010 != 0b000 { 'w' } else { '-' },
        if digit & 0b001 != 0b000 { 'x' } else { '-' }
    )
}

fn printfstyle(format: &str, args: HashMap<&str, String>) -> String {
    let mut result = format.to_string();
    for (arg, val) in &args {
        let regex = Regex::new(&format!(r"%(-?\d+)?{}", arg)).unwrap();
        result = regex.replace_all(&result, |caps: &Captures| {
            let mut valrep = &val[..];
            let vallen = valrep.len() as i16;
            let mut repl = String::new();
            let padding = if caps.at(1).unwrap_or("").starts_with("0") { '0' } else { ' ' };
            let mut width = if let Some(x) = caps.at(1) {
                                if let Ok(x) = i16::from_str_radix(x, 10) {
                                    x * -1
                                }
                                else {
                                    0
                                }
                            }
                            else {
                                0
                            };
            if width != 0 && width.abs() < vallen {
                valrep = &valrep[(vallen - width.abs()) as usize..];
                width = 0;
            }
            if width < 0 {
                for _ in width + vallen..0 {
                    repl.push(padding);
                }
            }
            repl.push_str(valrep);
            if width > 0 {
                for _ in 0..width - vallen {
                    repl.push(padding);
                }
            }
            repl
        });
    }
    result
}
