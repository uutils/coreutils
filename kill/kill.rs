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
use std::from_str::from_str;
use std::io::process::Process;

use getopts::{
    getopts,
    optopt,
    optflag,
	  optflagopt,
    usage,
};


static PROGNAME :&'static str = "kill";
static VERSION  :&'static str = "0.0.1";

static EXIT_OK  :i32 = 0;
static EXIT_ERR :i32 = 1;



#[deriving(Eq)]
pub enum Mode {
    Kill,
    Table,
    List,
    Help,
    Version,
}

static DEFAULT_SIGNAL:uint = 15;


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


/*


https://developer.apple.com/library/mac/documentation/Darwin/Reference/ManPages/man3/signal.3.html


No    Name         Default Action       Description
1     SIGHUP       terminate process    terminal line hangup
2     SIGINT       terminate process    interrupt program
3     SIGQUIT      create core image    quit program
4     SIGILL       create core image    illegal instruction
5     SIGTRAP      create core image    trace trap
6     SIGABRT      create core image    abort program (formerly SIGIOT)
7     SIGEMT       create core image    emulate instruction executed
8     SIGFPE       create core image    floating-point exception
9     SIGKILL      terminate process    kill program
10    SIGBUS       create core image    bus error
11    SIGSEGV      create core image    segmentation violation
12    SIGSYS       create core image    non-existent system call invoked
13    SIGPIPE      terminate process    write on a pipe with no reader
14    SIGALRM      terminate process    real-time timer expired
15    SIGTERM      terminate process    software termination signal
16    SIGURG       discard signal       urgent condition present on socket
17    SIGSTOP      stop process         stop (cannot be caught or ignored)
18    SIGTSTP      stop process         stop signal generated from keyboard
19    SIGCONT      discard signal       continue after stop
20    SIGCHLD      discard signal       child status has changed
21    SIGTTIN      stop process         background read attempted from control terminal
22    SIGTTOU      stop process         background write attempted to control terminal
23    SIGIO        discard signal       I/O is possible on a descriptor (see fcntl(2))
24    SIGXCPU      terminate process    cpu time limit exceeded (see setrlimit(2))
25    SIGXFSZ      terminate process    file size limit exceeded (see setrlimit(2))
26    SIGVTALRM    terminate process    virtual time alarm (see setitimer(2))
27    SIGPROF      terminate process    profiling timer alarm (see setitimer(2))
28    SIGWINCH     discard signal       Window size change
29    SIGINFO      discard signal       status request from keyboard
30    SIGUSR1      terminate process    User defined signal 1
31    SIGUSR2      terminate process    User defined signal 2

*/

#[cfg(target_os = "macos")]
static all_signals:[Signal<'static>, ..31] = [
	  Signal{ name: "HUP",    value:1  },
	  Signal{ name: "INT",    value:2  },
	  Signal{ name: "QUIT",   value:3  },
	  Signal{ name: "ILL",    value:4  },
	  Signal{ name: "TRAP",   value:5  },
	  Signal{ name: "ABRT",   value:6  },
	  Signal{ name: "EMT",    value:7  },
	  Signal{ name: "FPE",    value:8  },
	  Signal{ name: "KILL",   value:9  },
	  Signal{ name: "BUS",    value:10 },
	  Signal{ name: "SEGV",   value:11 },
	  Signal{ name: "SYS",    value:12 },
	  Signal{ name: "PIPE",   value:13 },
	  Signal{ name: "ALRM",   value:14 },
	  Signal{ name: "TERM",   value:15 },
	  Signal{ name: "URG",    value:16 },
	  Signal{ name: "STOP",   value:17 },
	  Signal{ name: "TSTP",   value:18 },
	  Signal{ name: "CONT",   value:19 },
	  Signal{ name: "CHLD",   value:20 },
	  Signal{ name: "TTIN",   value:21 },
	  Signal{ name: "TTOU",   value:22 },
	  Signal{ name: "IO",    value:23 },
	  Signal{ name: "XCPU",   value:24 },
	  Signal{ name: "XFSZ",   value:25 },
	  Signal{ name: "VTALRM", value:26 },
	  Signal{ name: "PROF",   value:27 },
	  Signal{ name: "WINCH",  value:28 },
	  Signal{ name: "INFO",   value:29 },
	  Signal{ name: "USR1",    value:30 },
	  Signal{ name: "USR2",    value:31 },
];

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
            error!("{}: {:s}", PROGNAME, e.to_err_msg());
						help(PROGNAME, usage);
						std::os::set_exit_status(1);
						return;
        },
    };


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
        Kill    => kill(matches.opt_str("signal").unwrap_or(~"9"), matches.free),
        Table   => table(),
        List    => list(matches.opt_str("list")),
        Help    => help(PROGNAME, usage),
        Version => version(),
    }
}

fn version() {
    println!("{} {}", PROGNAME, VERSION);
}

fn table() {

    /* Compute the maximum width of a signal number. */
    let mut signum = 1;
    let mut num_width = 1;
    while signum <= all_signals.len() / 10 {
        num_width += 1;
        signum *= 10;
    }
    let mut name_width = 0;
    /* Compute the maximum width of a signal name. */
    for s in all_signals.iter() {
        if s.name.len() > name_width {
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
				
        if (idx+1) % 7 == 0 {
            println!("");
        }

    }

}

fn print_signal(signal_name_or_value: ~str) {
	  for signal in all_signals.iter() {
			  if signal.name == signal_name_or_value  || ("SIG" + signal.name) == signal_name_or_value {
					  println!("{}", signal.value)
						sys_exit(EXIT_OK);
				} else if signal_name_or_value == signal.value.to_str() {
					  println!("{}", signal.name);
						sys_exit(EXIT_OK);
				}
		}
		println!("{}: unknown signal name {}", PROGNAME, signal_name_or_value)
		sys_exit(EXIT_ERR);
}

fn print_signals() {
	  let mut pos = 0;
    for (idx, signal) in all_signals.iter().enumerate() {
			  pos += signal.name.len();
			  print!("{}", signal.name);
		    if idx > 0 && pos > 73 {
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

fn signal_by_name_or_value(signal_name_or_value:~str) -> Option<uint> {
    for signal in all_signals.iter() {
			  let long_name = "SIG" + signal.name;
			  if signal.name == signal_name_or_value  || (signal_name_or_value == signal.value.to_str()) || (long_name == signal_name_or_value) {
					  return Some(signal.value);
				}
		}
		return None;
}

fn kill(signalname: ~str, pids: ~[~str]) {
		let optional_signal_value = signal_by_name_or_value(signalname.clone());
		let mut signal_value:uint = DEFAULT_SIGNAL;
		match optional_signal_value {
		    Some(x) => signal_value = x,
			  None => {
 	          println!("{}: unknown signal name {}", PROGNAME, signalname);
		    }
		}
    for pid in pids.iter() {
			  match from_str::<i32>(*pid) {
					  Some(x) => {

							  let result = Process::kill(x, signal_value as int);
								match result {
									Ok(t) => (),
									Err(e) => ()
								
								};
						},
					  None => {
							  println!("{}: failed to parse argument {}", PROGNAME, signalname);
								sys_exit(EXIT_ERR);
						},
				};
		}
}
