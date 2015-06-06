#![crate_name = "install"]
#![feature(plugin)]
#![feature(rustc_private)]
#![feature(collections)]
#![feature(associated_consts)]
#![feature(fs)]
#![feature(fs_ext)] 

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Matuesz Twaróg <implicent@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */


#[macro_use] extern crate rustc_bitflags;
extern crate collections;
extern crate getopts;
extern crate regex;
extern crate log;
extern crate rustc;

use std::os::unix::prelude::PermissionsExt;
use std::env::current_dir;
use std::ffi::OsString;
use std::path::PathBuf;
use std::boxed::Box;
use std::borrow::ToOwned;
use regex::Regex;
use std::collections::HashSet;
use collections::string::String;
use collections::vec::Vec;
use getopts::Options;
use std::fs;
use std::path::Path;
#[path = "../common/util.rs"]
#[macro_use]
mod util;


mod user {
    pub const USER  : u32 = 0b1;
    pub const GROUP : u32 = 0o10;
    pub const OTHER : u32 = 0o100;
    pub const ALL   : u32 = USER|GROUP|OTHER;
}



mod permission {
    pub const READ    : u32 = 0o400;
    pub const WRITE   : u32 = 0o200;
    pub const EXECUTE : u32 = 0o100;
}


enum Type {
    Add,
    Remove,
    Set,
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();
    
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("v", "version", "output version information and exit");
    opts.optopt("t", "target-directory", "Specify the destination directory", "");
    opts.optopt("m", "mode", "Set the file mode bits for the installed file or directory to mode", "");
    
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            crash!(1, "{}", e);
        },
    };
    
    if matches.opt_present("help") {
        print_usage(opts);
        return 0;
    }
    
    if matches.opt_present("version") {
        print_version();
        return 0;
    }
    
    let mut free = matches.free.clone();
    
    let dest_str: String = match matches.opt_str("target-directory") {
        Some(x) => x.to_owned(),
        None => {
            if free.len() <= 1 {
                show_error!("Missing TARGET argument.  Try --help.");
                return 1;
            } else {
                let tmp = free.pop();
                tmp.unwrap()
            }
        },
    };

    let dest = Path::new(&dest_str);
    let sources: Vec<Box<&Path>> = if free.len() <= 0 {
        println!("Missing SOURCE argument. Try --help.");
        show_error!("Missing SOURCE argument. Try --help.");
        return 1;
    } else {
        let mut tmp : Vec<Box<&Path>> = Vec::new();
        for i in 0..free.len() {
            if fs::metadata(Path::new(&free[i].clone())).is_err() {
                println!("cannot stat ‘{}’: No such file or directory {}", free[i], fs::metadata(Path::new(&free[i].clone())).is_err());
                show_error!("cannot stat ‘{}’: No such file or directory", free[i]);
                return 1;
            }
            let boxer = Box::new(Path::new(&free[i]));
            tmp.push(boxer);
        }
        tmp
    };
    
    let mode = match matches.opt_str("mode") {
        Some(x) => parse_mode(x),
        None => 0o755,
    };
    
    let is_dest_dir = match fs::metadata(&dest) {
        Ok(m) => m.is_dir(),
        Err(_) => false
    };
    
    println!("is dest dir {}", is_dest_dir);
    
    if matches.opt_present("target-directory") || sources.len() > 1  || is_dest_dir {
        println!("many files");
        files_to_directory(sources, dest, mode);
    } else {
        println!("one file {} {}", (*sources[0]).display(), dest.display());
        file_to_file(&*sources[0], dest, mode);
    }
    0
}

fn file_to_file(source: &Path, dest: &Path, mode: u32) {
    let real_source = real(source);
    let real_dest = real(dest);
    
    println!("realll {:?} {:?} {}", real_source, real_dest, real_source==real_dest);
    
    if real_source == real_dest {
        crash!(1, "{0} and {1} are the same file", source.display(), dest.display());
    }
    
    match fs::copy(source, dest) {
        Ok(_) => (),
        Err(e) => {
            crash!(1, "{}", e);
        },
    }
    
   
    let mut permissions = fs::metadata(dest).unwrap().permissions();
    permissions.set_mode(mode);
    match fs::set_permissions(dest, permissions) {
        Ok(m) => m,
        Err(e) => {
            crash!(1, "{}", e);
        },
    }
}

fn files_to_directory(sources : Vec<Box<&Path>>, dest : &Path, mode: u32) {
    match fs::metadata(dest) {
        Ok(m) => if !m.is_dir() {
            crash!(1, "failed to access ‘{}’: No such file or directory", dest.to_str());
        },
        Err(_) => {
            crash!(1, "target ‘{}’ is not a directory", dest.display());
        }
    };
    
    let mut set = HashSet::new();
    
    for i in 0..sources.len() {
        let mut stat = fs::metadata(&*sources[i]);
        if stat.is_ok() && stat.unwrap().is_dir() {
            println!("install: omitting directory ‘{}’", sources[i].display());
            continue;
        }
        let mut tmp_dest_buf = dest.to_path_buf().clone().to_owned();
        tmp_dest_buf.push(match sources[i].file_name() {
            Some(m) => m,
            None => unreachable!(),
        });
        let tmp_dest : &Path = tmp_dest_buf.as_path();
        stat = fs::metadata(&tmp_dest);
        if stat.is_ok() && stat.unwrap().is_dir() {
            println!("install: cannot overwrite directory ‘{}’ with non-directory", tmp_dest.display());
            continue;
        }
        
        let real_source = real(&tmp_dest);
        let real_dest = real(&*sources[i]);
        
        println!("realll  {:?}   {:?}", real_source, real_dest);
        
        if real_source == real_dest {
            println!("install: {0} and {1} are the same file", sources[i].display(), tmp_dest.display());
            continue;
        }
        
        if set.contains(&real_dest){
            println!("install: will not overwrite just-created ‘{}’ with ‘{}’", tmp_dest.display(), sources[i].display());
            continue;
        }
        
        match fs::copy(&*sources[i], &*tmp_dest) {
            Ok(m) => {
                set.insert(real_dest);
                m
            },
            Err(e) => {
                crash!(1, "{}", e);
            },
        };
        
        let mut permissions = fs::metadata(tmp_dest).unwrap().permissions();
        permissions.set_mode(mode);
        match fs::set_permissions(tmp_dest, permissions) {
            Ok(m) => m,
            Err(e) => {
                crash!(1, "{}", e);
            },
        }
    }
}

fn parse_mode(s : String) -> u32 {
    println!("mode parsing");
    
    let mut out : u32 = 0;
    let split: Vec<&str> = s.split(',').collect();
    let regexp = Regex::new(r"^[ugoa]*[-=+][rwx]*$").unwrap();
    for i in split.iter() {
    
        if !regexp.is_match(i) {
            crash!(1, "invalid mode '{}'", s);
        }
        
        let mut user = 0;
        let mut permission = 0;
        let re = Regex::new(r"[-=+]").unwrap();
        let sp: Vec<&str> = re.split(i).collect();
        for c in sp[0].chars() {
            user = user | match c {
                'u' => user::USER,
                'g' => user::GROUP,
                'o' => user::OTHER,
                'a' => user::ALL,
                _   => unreachable!(),
            };
        }
        for c in sp[1].chars() {
            permission = permission | match c {
                'r' => permission::READ,
                'w' => permission::WRITE,
                'x' => permission::EXECUTE,
                _   => unreachable!(),
            };
        }
        
        println!("user {}, perm {}", user, permission);
        let mut cap = match re.captures(i) {
            Some(s) => s.at(0).unwrap().chars(),
            None => unreachable!(),
        };
        
        let operator = match cap.next() {
            Some(s) => match s {
                '-' => Type::Remove,
                '=' => Type::Set,
                '+' => Type::Add,
                _   => unreachable!(),
            },
            None => unreachable!(),
        };
        
        for u in vec![user::USER, user::GROUP, user::OTHER].into_iter() {
            if u & user != 0 {
                match operator {
                    Type::Set => out = out & !(permission::READ/u | permission::WRITE/u | permission::EXECUTE/u),
                    _         => (),
                };
                for p in vec![permission::READ, permission::WRITE, permission::EXECUTE].into_iter() {
                    if p & permission != 0 {
                        match operator {
                            Type::Remove => out = out & !(p/u),
                            Type::Set    => out = out | (p/u),
                            Type::Add    => out = out | (p/u),
                        };
                    }
                }
            }
        }
    }
    out
}

fn real(path: & Path) -> Box<PathBuf> {
    let mut real_path = current_dir().unwrap();
    
    for component in path.components() {
    	let mut real_path_clone = real_path.clone();
    	real_path_clone.push(component.as_os_str());
    	
    	let next : OsString = match fs::read_link(&real_path_clone) {
    	    Ok(m) => {  println!("here");
    	                m.file_name().unwrap().to_owned()},
    	    Err(_) => (*component.as_os_str()).to_os_string()  
    	};
    	real_path.push(next);
    	println!("pav {} {}", real_path.display(), real_path_clone.display());
    }

    let bbox = Box::new(Path::new(real_path.as_path()).to_owned());
    bbox
}

fn print_usage(opts: Options) {
    let msg = format!("Usage: install [OPTION]... [-T] SOURCE DEST
  or:  install [OPTION]... SOURCE... DIRECTORY
  or:  install [OPTION]... -t DIRECTORY SOURCE...
  or:  install [OPTION]... -d DIRECTORY...
    ");
    println!("{}",  opts.usage(&msg));
}

fn print_version() {
	println!("install version 1.0.0");
}
