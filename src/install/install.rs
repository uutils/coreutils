#![crate_name = "install"]
#![feature(macro_rules)]
#![feature(phase)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Matuesz Twaróg <implicent@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

#![allow(missing_copy_implementations)] //to be removed in later stage
#![allow(unused_variables)]  //to be removed in later stage 

extern crate collections;
extern crate getopts;
#[phase(plugin, link)] extern crate log;
extern crate regex;
#[phase(plugin)] extern crate regex_macros;

use std::os::make_absolute;
use std::collections::HashSet;
use std::collections::HashMap;
use collections::string::String;
use collections::vec::Vec;
use getopts::{
    getopts,
    optflag,
    optopt,
    OptGroup,
};
use std::io::{
    fs,
    FilePermission,
    FileType,
    GROUP_EXECUTE,
    GROUP_READ,
    GROUP_WRITE,
    OTHER_EXECUTE,
    OTHER_READ,
    OTHER_WRITE,
    USER_EXECUTE,
    USER_READ,
    USER_RWX,
    USER_WRITE,
};
#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "install";

bitflags! {
    flags User: u32 {
        const USER  = 0x00000001,
        const GROUP = 0x00000010,
        const OTHER = 0x00000100,
    }
}

bitflags! {
    flags Permission: u32 {
        const READ    = 0x00000001,
        const WRITE   = 0x00000010,
        const EXECUTE = 0x00000100,
    }
}

enum Type {
    Add,
    Remove,
    Set,
}

struct Action {
    t: Type,
    p: FilePermission,
}

impl Action {
    fn apply_on(&self, p: &mut FilePermission) {
        match self.t {
            Type::Add => p.insert(self.p),
            Type::Remove => p.remove(self.p),
            Type::Set => {
                p.remove(FilePermission::all());
                p.insert(self.p)
            },
        }
    }
}

pub fn uumain(args: Vec<String>) -> int {
    let program = args[0].clone();
    let opts = [
        optflag("h", "help", "display this help and exit"),
        optflag("v", "version", "output version information and exit"),
        optopt("t", "target-directory", "Specify the destination directory", ""),
        optopt("m", "mode", "Set the file mode bits for the installed file or directory to mode", ""),
    ];
    let matches = match getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(e) => {
            crash!(1, "{}", e);
        },
    };
    
    if matches.opt_present("help") {
        print_usage(&opts);
        return 0;
    }
    
    if matches.opt_present("version") {
        print_version();
        return 0;
    }
    
    let mut free = matches.free.clone();
    
    let dest: Path = match matches.opt_str("target-directory") {
        Some(x) => Path::new(x),
        None => {
            if free.len() <= 1 {
                show_error!("Missing TARGET argument.  Try --help.");
                return 1;
            } else {
                let tmp = free.pop();
                Path::new(tmp.unwrap())
            }
        },
    };
    let sources: Vec<Path> = if free.len() <= 0 {
        show_error!("Missing SOURCE argument. Try --help.");
        return 1;
    } else {
        let mut tmp : Vec<Path> = Vec::new();
        for i in range(0, free.len()) {
            if fs::stat(&Path::new(free[i].clone())).is_err() {
                show_error!("cannot stat ‘{}’: No such file or directory", free[i]);
                return 1;
            }
            tmp.push(Path::new(free[i].clone()));
        }
        tmp
    };
    
    let mode = match matches.opt_str("mode") {
        Some(x) => parse_mode(x),
        None => USER_RWX | GROUP_READ | GROUP_EXECUTE | OTHER_READ | OTHER_EXECUTE,
    };
    
    let is_dest_dir = match fs::stat(&dest) {
        Ok(m) => m.kind == std::io::FileType::Directory,
        Err(_) => false
    };
    
    if matches.opt_present("target-directory") || sources.len() > 1  || is_dest_dir {
        files_to_directory(sources, dest, mode);
    } else {
        file_to_file(sources[0].clone(), dest, mode);
    }
    0
}

fn file_to_file(source: Path, dest: Path, mode: FilePermission) {
    let (real_source, real_dest) = real(&source, &dest);
    
    if real_source == real_dest {
        crash!(1, "{0} and {1} are the same file", source.display(), dest.display());
    }
    
    match fs::copy(&source, &dest) {
        Ok(m) => m,
        Err(e) => {
            crash!(1, "{}", e);
        },
    }
    
    match fs::chmod(&dest, mode) {
        Ok(m) => m,
        Err(e) => {
            crash!(1, "{}", e);
        },
    }
}

fn files_to_directory(sources : Vec<Path>, dest : Path, mode: FilePermission) {
    match fs::stat(&dest) {
        Ok(m) => if m.kind != FileType::Directory {
            crash!(1, "failed to access ‘{}’: No such file or directory", dest.display());
        },
        Err(_) => {
            crash!(1, "target ‘{}’ is not a directory", dest.display());
        }
    };
    
    let mut set = HashSet::new();
    
    for i in range(0, sources.len()) {
        let mut stat = fs::stat(&sources[i]);
        if stat.is_ok() && stat.unwrap().kind == FileType::Directory {
            println!("install: omitting directory ‘{}’", sources[i].display());
            continue;
        }
        let mut tmp_dest = dest.clone();
        tmp_dest.push(match sources[i].filename_str() {
            Some(m) => m,
            None => unreachable!(),
        });
        
        stat = fs::stat(&tmp_dest);
        if stat.is_ok() && stat.unwrap().kind == FileType::Directory {
            println!("install: cannot overwrite directory ‘{}’ with non-directory", tmp_dest.display());
            continue;
        }
        
        let (real_source, real_dest) = real(&tmp_dest, &sources[i]);
        
        if real_source == real_dest {
            println!("install: {0} and {1} are the same file", sources[i].display(), tmp_dest.display());
            continue;
        }
        
        if set.contains(&real_dest){
            println!("install: will not overwrite just-created ‘{}’ with ‘{}’", tmp_dest.display(), sources[i].display());
            continue;
        }
        
        match fs::copy(&sources[i], &tmp_dest) {
            Ok(m) => {
                set.insert(real_dest);
                m
            },
            Err(e) => {
                crash!(1, "{}", e);
            },
        };
        
        match fs::chmod(&tmp_dest, mode) {
            Ok(m) => m,
            Err(e) => {
                crash!(1, "{}", e);
            },
        }
    }
}

fn parse_mode(s : String) -> FilePermission {
    let mut map = HashMap::new();
    map.insert((USER, READ), USER_READ);
    map.insert((USER, WRITE), USER_WRITE);
    map.insert((USER, EXECUTE), USER_EXECUTE);
    map.insert((GROUP, READ), GROUP_READ);
    map.insert((GROUP, WRITE), GROUP_WRITE);
    map.insert((GROUP, EXECUTE), GROUP_EXECUTE);
    map.insert((OTHER, READ), OTHER_READ);
    map.insert((OTHER, WRITE), OTHER_WRITE);
    map.insert((OTHER, EXECUTE), OTHER_EXECUTE);
    
    let mut out = FilePermission::empty();
    let split: Vec<&str> = s.as_slice().split(',').collect();
    let regexp = regex!(r"^[ugoa]*[-=+][rwx]*$");
    for i in split.iter() {
    
        if !regexp.is_match(i.as_slice()) {
            crash!(1, "invalid mode '{}'", s);
        }
        
        let mut user = User::empty();
        let mut permission = Permission::empty();
        let re = regex!(r"[-=+]");
        let sp: Vec<&str> = re.split(i.as_slice()).collect();
        for c in sp[0].chars() {
            user = user | match c {
                'u' => USER,
                'g' => GROUP,
                'o' => OTHER,
                'a' => User::all(),
                _   => unreachable!(),
            };
        }
        for c in sp[1].chars() {
            permission = permission | match c {
                'r' => READ,
                'w' => WRITE,
                'x' => EXECUTE,
                _   => unreachable!(),
            };
        }
        
        let mut file_permissions = FilePermission::empty();
        
        for u in vec![USER, GROUP, OTHER].into_iter() {
            if u & user != User::empty() {
                for p in vec![READ, WRITE, EXECUTE].into_iter() {
                    if p & permission != Permission::empty() {
                        file_permissions.insert(*map.get(&(u.clone(), p.clone())).unwrap());
                    }
                }
            }
        }
        
        let mut cap = match re.captures(i.as_slice()) {
            Some(s) => s.at(0).chars(),
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
        
        let action = Action{ t: operator, p: file_permissions };
        action.apply_on(&mut out);
    }
    out
}

fn real(source: &Path, dest: &Path) -> (Path, Path) {
    let real_source = match make_absolute(source) {
        Ok(m) => m,
        Err(e) => {
            crash!(1, "{}", e);
        },
    };
    
    let real_dest = match make_absolute(dest) {
        Ok(m) => m,
        Err(e) => {
            crash!(1, "{}", e);
        },
    };
    (real_source, real_dest)
}

fn print_usage(opts: &[OptGroup]) {
    let msg = format!("Usage: install [OPTION]... [-T] SOURCE DEST
  or:  install [OPTION]... SOURCE... DIRECTORY
  or:  install [OPTION]... -t DIRECTORY SOURCE...
  or:  install [OPTION]... -d DIRECTORY...
    ");
    println!("{}",  getopts::usage(msg.as_slice(), opts));
}

fn print_version() {
	println!("install version 1.0.0");
}