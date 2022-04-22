// This file is part of the uutils coreutils package.
//
// (c) Maciej Dziardziel <fiedzia@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// spell-checker:ignore (vars/api) fcntl setrlimit setitimer
// spell-checker:ignore (vars/signals) ABRT ALRM CHLD SEGV SIGABRT SIGALRM SIGBUS SIGCHLD SIGCONT SIGEMT SIGFPE SIGHUP SIGILL SIGINFO SIGINT SIGIO SIGIOT SIGKILL SIGPIPE SIGPROF SIGQUIT SIGSEGV SIGSTOP SIGSYS SIGTERM SIGTRAP SIGTSTP SIGTTIN SIGTTOU SIGURG SIGUSR SIGVTALRM SIGWINCH SIGXCPU SIGXFSZ STKFLT TSTP TTIN TTOU VTALRM XCPU XFSZ

pub static DEFAULT_SIGNAL: usize = 15;

/*

Linux Programmer's Manual

 1 HUP      2 INT      3 QUIT     4 ILL      5 TRAP     6 ABRT     7 BUS
 8 FPE      9 KILL    10 USR1    11 SEGV    12 USR2    13 PIPE    14 ALRM
15 TERM    16 STKFLT  17 CHLD    18 CONT    19 STOP    20 TSTP    21 TTIN
22 TTOU    23 URG     24 XCPU    25 XFSZ    26 VTALRM  27 PROF    28 WINCH
29 POLL    30 PWR     31 SYS


*/

#[cfg(any(target_os = "linux", target_os = "android"))]
pub static ALL_SIGNALS: [&str; 32] = [
    "EXIT", "HUP", "INT", "QUIT", "ILL", "TRAP", "ABRT", "BUS", "FPE", "KILL", "USR1", "SEGV",
    "USR2", "PIPE", "ALRM", "TERM", "STKFLT", "CHLD", "CONT", "STOP", "TSTP", "TTIN", "TTOU",
    "URG", "XCPU", "XFSZ", "VTALRM", "PROF", "WINCH", "POLL", "PWR", "SYS",
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

#[cfg(any(target_vendor = "apple", target_os = "freebsd"))]
pub static ALL_SIGNALS: [&str; 32] = [
    "EXIT", "HUP", "INT", "QUIT", "ILL", "TRAP", "ABRT", "EMT", "FPE", "KILL", "BUS", "SEGV",
    "SYS", "PIPE", "ALRM", "TERM", "URG", "STOP", "TSTP", "CONT", "CHLD", "TTIN", "TTOU", "IO",
    "XCPU", "XFSZ", "VTALRM", "PROF", "WINCH", "INFO", "USR1", "USR2",
];

pub fn signal_by_name_or_value(signal_name_or_value: &str) -> Option<usize> {
    if let Ok(value) = signal_name_or_value.parse() {
        if is_signal(value) {
            return Some(value);
        } else {
            return None;
        }
    }
    let signal_name = signal_name_or_value.trim_start_matches("SIG");

    ALL_SIGNALS.iter().position(|&s| s == signal_name)
}

pub fn is_signal(num: usize) -> bool {
    num < ALL_SIGNALS.len()
}

pub fn signal_name_by_value(signal_value: usize) -> Option<&'static str> {
    ALL_SIGNALS.get(signal_value).copied()
}

#[test]
fn signal_by_value() {
    assert_eq!(signal_by_name_or_value("0"), Some(0));
    for (value, _signal) in ALL_SIGNALS.iter().enumerate() {
        assert_eq!(signal_by_name_or_value(&value.to_string()), Some(value));
    }
}

#[test]
fn signal_by_short_name() {
    for (value, signal) in ALL_SIGNALS.iter().enumerate() {
        assert_eq!(signal_by_name_or_value(signal), Some(value));
    }
}

#[test]
fn signal_by_long_name() {
    for (value, signal) in ALL_SIGNALS.iter().enumerate() {
        assert_eq!(
            signal_by_name_or_value(&format!("SIG{}", signal)),
            Some(value)
        );
    }
}

#[test]
fn name() {
    for (value, signal) in ALL_SIGNALS.iter().enumerate() {
        assert_eq!(signal_name_by_value(value), Some(*signal));
    }
}
