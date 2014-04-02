#[crate_id(name="kill", vers="1.0.0", author="Maciej Dziardziel")];
#[feature(macro_rules)];
#[feature(phase)];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Maciej Dziardziel <fiedzia@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

/*
 Linux Programmer's Manual

   1 HUP      2 INT      3 QUIT     4 ILL      5 TRAP     6 ABRT     7 BUS
   8 FPE      9 KILL    10 USR1    11 SEGV    12 USR2    13 PIPE    14 ALRM
  15 TERM    16 STKFLT  17 CHLD    18 CONT    19 STOP    20 TSTP    21 TTIN
  22 TTOU    23 URG     24 XCPU    25 XFSZ    26 VTALRM  27 PROF    28 WINCH
  29 POLL    30 PWR     31 SYS
 
*/
extern crate getopts;
extern crate collections;
extern crate serialize;
#[phase(syntax, link)] extern crate log;

use std::os;
use std::io;
use std::cast;
use std::hash::Hash;
use std::io::fs;
use std::fmt::format_unsafe;
use collections::HashMap;
use collections::enum_set::{EnumSet, CLike};


use getopts::{
    getopts,
    optopt,
    optflag,
	  optflagopt,
    usage,
};

#[deriving(Eq)]
pub enum Mode {
    Kill,
    Table,
    List,
    Help,
    Version,
}

struct Signal<'a> { name:&'a str, value: uint}

#[cfg(target_os = "linux")]
static all_signals:[Signal<'static>, ..31] = [
	  Signal{ name: "HUP",    value:1  },
	  Signal{ name: "INT",    value:2  },
	  Signal{ name: "QUIT",   value:3  },
	  Signal{ name: "ILL",    value:4  },
	  Signal{ name: "TRAP",   value:5  },
	  Signal{ name: "ABRT",   value:6  },
	  Signal{ name: "BUS",    value:7  },
	  Signal{ name: "FPE",    value:8  },
	  Signal{ name: "KILL",   value:9  },
	  Signal{ name: "USR1",   value:10 },
	  Signal{ name: "SEGV",   value:11 },
	  Signal{ name: "USR2",   value:12 },
	  Signal{ name: "PIPE",   value:13 },
	  Signal{ name: "ALRM",   value:14 },
	  Signal{ name: "TERM",   value:15 },
	  Signal{ name: "STKFLT", value:16 },
	  Signal{ name: "CHLD",   value:17 },
	  Signal{ name: "CONT",   value:18 },
	  Signal{ name: "STOP",   value:19 },
	  Signal{ name: "TSTP",   value:20 },
	  Signal{ name: "TTIN",   value:21 },
	  Signal{ name: "TTOU",   value:22 },
	  Signal{ name: "URG",    value:23 },
	  Signal{ name: "XCPU",   value:24 },
	  Signal{ name: "XFSZ",   value:25 },
	  Signal{ name: "VTALRM", value:26 },
	  Signal{ name: "PROF",   value:27 },
	  Signal{ name: "WINCH",  value:28 },
	  Signal{ name: "POLL",   value:29 },
	  Signal{ name: "PWR",    value:30 },
	  Signal{ name: "SYS",    value:31 },
];



static PROGNAME :&'static str = "kill"; //args[0].clone();

//global exit with status
fn sys_exit(status:std::libc::c_int){
    unsafe {std::libc::exit(status) }
}


fn main() {
    let args = os::args();

    let opts = ~[
        optflag("h", "help", "display this help and exit"),
        optflag("V", "version", "output version information and exit"),
        optopt("s", "signal", "specify the <signal> to be sent", "SIGNAL"),
			  optflagopt("l", "list", "list all signal names, or convert one to a name", "LIST"),
        optflag("L", "table", "list all signal names in a nice table"),
    ];

    let usage = usage("[options] <pid> [...]", opts);


    let matches = match getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(e) => {
            error!("kill: {:s}", e.to_err_msg());
						help(PROGNAME, usage);
						std::os::set_exit_status(1);
						return;
        },
    };


    for (s in matches.free.iter()) {
			println!("{}". s);
		}

    let mode = if matches.opt_present("version") {
        Version
    } else if matches.opt_present("help") {
        Help
    } else if matches.opt_present("table") {
                 Table
    } else if matches.opt_present("list") {
                 List
    } else {
        Kill
    };

    match mode {
        Kill    => kill(matches),
        Table   => table(),
        List    => list(matches.opt_str("list")),
        Help    => help(PROGNAME, usage),
        Version => version(),
    }
}

fn version() {
    println!("kill 1.0.0");
}

fn table() {

    /* Compute the maximum width of a signal number. */
    let mut signum = 1;
    let mut num_width = 1;
    while (signum <= all_signals.len() / 10){
        num_width += 1;
        signum *= 10;
    }
    let mut name_width = 0;
    /* Compute the maximum width of a signal name. */
    for s in all_signals.iter() {
        if (s.name.len() > name_width) {
            name_width = s.name.len()
        }
    }

    for (idx, signal) in all_signals.iter().enumerate() {
			  //let f: [& str, ..1] = ["{}"];
				//let f:[std::fmt::rt::Piece, ..1];
        print!("{0: >#2} {1: <#8}", idx+1, signal.name);
				//TODO: obtain max signal width here
				//let args: [std::fmt::Argument, ..1];
				//let f = "{0} {1}";
				//let mut parser = std::fmt::parse::Parser::new(f);
				//let mut args = parser.collect();
				//unsafe {
	      //  let s = format_unsafe(args, [1,2]);
				//}
				//print!(f, idx+1, signame);
				
        if ((idx+1) % 7 == 0) {
            println!("");
        }

    }

}

fn print_signal(signal_name_or_value: ~str) {
	  for signal in all_signals.iter() {
			  if(signal.name == signal_name_or_value) {
					  println!("{}", signal.value)
						sys_exit(0);
				} else if (signal_name_or_value == signal.value.to_str()) {
					  println!("{}", signal.name);
						sys_exit(0);
				}
		}
		println!("{}: unknown signal name {}", PROGNAME, signal_name_or_value)
		sys_exit(1);
	  //let optnum:Option<int> = from_str(signal);
		//if optnum.is_some() {
		//	  let num = optnum.unwrap();
		//}
}

fn print_signals() {
	  let mut pos = 0;
    for (idx, signal) in all_signals.iter().enumerate() {
			  pos += signal.name.len();
			  print!("{}", signal.name);
		    if (idx > 0 && pos > 73) {
							  println!("");
							  pos = 0;
				} else {
							  pos += 1;
								print!(" ");
				} 
	  }
}

fn list(arg: Option<~str>) {
    match arg {
			Some(x) => print_signal(x),
			None => print_signals(),
		};
}


fn help(progname: &str, usage: &str) {
    let msg = format!("Usage: \n {0} {1}", progname, usage);
    println!("{}", msg);
}


fn kill(matches: getopts::Matches) {
}


fn copy(matches: getopts::Matches) {
    let sources = if matches.free.len() < 1 {
        error!("error: Missing SOURCE argument. Try --help.");
        fail!()
    } else {
        // All but the last argument:
        matches.free.slice(0, matches.free.len() - 2)
            .map(|arg| ~Path::new(arg.clone()))
    };
    let dest = if matches.free.len() < 2 {
        error!("error: Missing DEST argument. Try --help.");
        fail!()
    } else {
        // Only the last argument:
        ~Path::new(matches.free[matches.free.len() - 1].clone())
    };

    assert!(sources.len() >= 1);

    if sources.len() == 1 {
        let source = sources[0].clone();
        let same_file = match paths_refer_to_same_file(source, dest) {
            Ok(b)  => b,
            Err(e) => if e.kind == io::FileNotFound {
                false
            } else {
                error!("error: {:s}", e.to_str());
                fail!()
            }
        };

        if same_file {
            error!("error: \"{:s}\" and \"{:s}\" are the same file",
                source.display().to_str(),
                dest.display().to_str());
            fail!();
        }

        let io_result = fs::copy(source, dest);

        if io_result.is_err() {
            let err = io_result.unwrap_err();
            error!("error: {:s}", err.to_str());
            fail!();
        }
    } else {
        if fs::stat(dest).unwrap().kind != io::TypeDirectory {
            error!("error: TARGET must be a directory");
            fail!();
        }

        for source in sources.iter() {
            if fs::stat(*source).unwrap().kind != io::TypeFile {
                error!("error: \"{:s}\" is not a file", source.display().to_str());
                continue;
            }

            let mut full_dest = dest.clone();

            full_dest.push(source.filename_str().unwrap());

            println!("{:s}", full_dest.display().to_str());

            let io_result = fs::copy(*source, full_dest);

            if io_result.is_err() {
                let err = io_result.unwrap_err();
                error!("error: {:s}", err.to_str());
                fail!()
            }
        }
    }
}

pub fn paths_refer_to_same_file(p1: &Path, p2: &Path) -> io::IoResult<bool> {
    let mut raw_p1 = ~p1.clone();
    let mut raw_p2 = ~p2.clone();

    let p1_lstat = match fs::lstat(raw_p1) {
        Ok(stat) => stat,
        Err(e)   => return Err(e),
    };

    let p2_lstat = match fs::lstat(raw_p2) {
        Ok(stat) => stat,
        Err(e)   => return Err(e),
    };

    // We have to take symlinks and relative paths into account.
    if p1_lstat.kind == io::TypeSymlink {
        raw_p1 = ~fs::readlink(raw_p1).unwrap();
    }
    raw_p1 = ~os::make_absolute(raw_p1);

    if p2_lstat.kind == io::TypeSymlink {
        raw_p2 = ~fs::readlink(raw_p2).unwrap();
    }
    raw_p2 = ~os::make_absolute(raw_p2);

    Ok(raw_p1 == raw_p2)
}
