/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Maciej Dziardziel <fiedzia@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

pub static DEFAULT_SIGNAL:usize= 15;


pub struct Signal<'a> { pub name:&'a str, pub value: usize}

/*

Linux Programmer's Manual

 1 HUP      2 INT      3 QUIT     4 ILL      5 TRAP     6 ABRT     7 BUS
 8 FPE      9 KILL    10 USR1    11 SEGV    12 USR2    13 PIPE    14 ALRM
15 TERM    16 STKFLT  17 CHLD    18 CONT    19 STOP    20 TSTP    21 TTIN
22 TTOU    23 URG     24 XCPU    25 XFSZ    26 VTALRM  27 PROF    28 WINCH
29 POLL    30 PWR     31 SYS


*/

#[cfg(target_os = "linux")]
pub static ALL_SIGNALS:[Signal<'static>; 31] = [
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

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
pub static ALL_SIGNALS:[Signal<'static>; 31] = [
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
    Signal{ name: "IO",     value:23 },
    Signal{ name: "XCPU",   value:24 },
    Signal{ name: "XFSZ",   value:25 },
    Signal{ name: "VTALRM", value:26 },
    Signal{ name: "PROF",   value:27 },
    Signal{ name: "WINCH",  value:28 },
    Signal{ name: "INFO",   value:29 },
    Signal{ name: "USR1",   value:30 },
    Signal{ name: "USR2",   value:31 },
];

pub fn signal_by_name_or_value(signal_name_or_value: &str) -> Option<usize> {
    if signal_name_or_value == "0" {
        return Some(0);
    }
    for signal in &ALL_SIGNALS {
        let long_name = format!("SIG{}", signal.name);
        if signal.name == signal_name_or_value  || (signal_name_or_value == signal.value.to_string()) || (long_name == signal_name_or_value) {
            return Some(signal.value);
        }
    }
    None
}

#[inline(always)]
pub fn is_signal(num: usize) -> bool {
    num < ALL_SIGNALS.len()
}
