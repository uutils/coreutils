// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars/api) fcntl setrlimit setitimer rubout pollable sysconf
// spell-checker:ignore (vars/signals) ABRT ALRM CHLD SEGV SIGABRT SIGALRM SIGBUS SIGCHLD SIGCONT SIGDANGER SIGEMT SIGFPE SIGHUP SIGILL SIGINFO SIGINT SIGIO SIGIOT SIGKILL SIGMIGRATE SIGMSG SIGPIPE SIGPRE SIGPROF SIGPWR SIGQUIT SIGSEGV SIGSTOP SIGSYS SIGTALRM SIGTERM SIGTRAP SIGTSTP SIGTHR SIGTTIN SIGTTOU SIGURG SIGUSR SIGVIRT SIGVTALRM SIGWINCH SIGXCPU SIGXFSZ STKFLT PWR THR TSTP TTIN TTOU VIRT VTALRM XCPU XFSZ SIGCLD SIGPOLL SIGWAITING SIGAIOCANCEL SIGLWP SIGFREEZE SIGTHAW SIGCANCEL SIGLOST SIGXRES SIGJVM SIGRTMIN SIGRT SIGRTMAX TALRM AIOCANCEL XRES RTMIN RTMAX

//! This module provides a way to handle signals in a platform-independent way.
//! It provides a way to convert signal names to their corresponding values and vice versa.
//! It also provides a way to ignore the SIGINT signal and enable pipe errors.

#[cfg(unix)]
use nix::errno::Errno;
#[cfg(unix)]
use nix::sys::signal::{
    signal, SigHandler::SigDfl, SigHandler::SigIgn, Signal::SIGINT, Signal::SIGPIPE,
};

/// The default signal value.
pub static DEFAULT_SIGNAL: usize = 15;

/*

Linux Programmer's Manual

 1 HUP      2 INT      3 QUIT     4 ILL      5 TRAP     6 ABRT     7 BUS
 8 FPE      9 KILL    10 USR1    11 SEGV    12 USR2    13 PIPE    14 ALRM
15 TERM    16 STKFLT  17 CHLD    18 CONT    19 STOP    20 TSTP    21 TTIN
22 TTOU    23 URG     24 XCPU    25 XFSZ    26 VTALRM  27 PROF    28 WINCH
29 POLL    30 PWR     31 SYS


*/

/// The list of all signals.
#[cfg(any(target_os = "linux", target_os = "android", target_os = "redox"))]
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

/*

     The following signals are defined in NetBSD:

     SIGHUP           1     Hangup
     SIGINT           2     Interrupt
     SIGQUIT          3     Quit
     SIGILL           4     Illegal instruction
     SIGTRAP          5     Trace/BPT trap
     SIGABRT          6     Abort trap
     SIGEMT           7     EMT trap
     SIGFPE           8     Floating point exception
     SIGKILL          9     Killed
     SIGBUS           10    Bus error
     SIGSEGV          11    Segmentation fault
     SIGSYS           12    Bad system call
     SIGPIPE          13    Broken pipe
     SIGALRM          14    Alarm clock
     SIGTERM          15    Terminated
     SIGURG           16    Urgent I/O condition
     SIGSTOP          17    Suspended (signal)
     SIGTSTP          18    Suspended
     SIGCONT          19    Continued
     SIGCHLD          20    Child exited, stopped or continued
     SIGTTIN          21    Stopped (tty input)
     SIGTTOU          22    Stopped (tty output)
     SIGIO            23    I/O possible
     SIGXCPU          24    CPU time limit exceeded
     SIGXFSZ          25    File size limit exceeded
     SIGVTALRM        26    Virtual timer expired
     SIGPROF          27    Profiling timer expired
     SIGWINCH         28    Window size changed
     SIGINFO          29    Information request
     SIGUSR1          30    User defined signal 1
     SIGUSR2          31    User defined signal 2
     SIGPWR           32    Power fail/restart
*/

#[cfg(target_os = "netbsd")]
pub static ALL_SIGNALS: [&str; 33] = [
    "EXIT", "HUP", "INT", "QUIT", "ILL", "TRAP", "ABRT", "EMT", "FPE", "KILL", "BUS", "SEGV",
    "SYS", "PIPE", "ALRM", "TERM", "URG", "STOP", "TSTP", "CONT", "CHLD", "TTIN", "TTOU", "IO",
    "XCPU", "XFSZ", "VTALRM", "PROF", "WINCH", "INFO", "USR1", "USR2", "PWR",
];

/*

     The following signals are defined in OpenBSD:

     SIGHUP       terminate process    terminal line hangup
     SIGINT       terminate process    interrupt program
     SIGQUIT      create core image    quit program
     SIGILL       create core image    illegal instruction
     SIGTRAP      create core image    trace trap
     SIGABRT      create core image    abort(3) call (formerly SIGIOT)
     SIGEMT       create core image    emulate instruction executed
     SIGFPE       create core image    floating-point exception
     SIGKILL      terminate process    kill program (cannot be caught or
                                       ignored)
     SIGBUS       create core image    bus error
     SIGSEGV      create core image    segmentation violation
     SIGSYS       create core image    system call given invalid argument
     SIGPIPE      terminate process    write on a pipe with no reader
     SIGALRM      terminate process    real-time timer expired
     SIGTERM      terminate process    software termination signal
     SIGURG       discard signal       urgent condition present on socket
     SIGSTOP      stop process         stop (cannot be caught or ignored)
     SIGTSTP      stop process         stop signal generated from keyboard
     SIGCONT      discard signal       continue after stop
     SIGCHLD      discard signal       child status has changed
     SIGTTIN      stop process         background read attempted from control
                                       terminal
     SIGTTOU      stop process         background write attempted to control
                                       terminal
     SIGIO        discard signal       I/O is possible on a descriptor (see
                                       fcntl(2))
     SIGXCPU      terminate process    CPU time limit exceeded (see
                                       setrlimit(2))
     SIGXFSZ      terminate process    file size limit exceeded (see
                                       setrlimit(2))
     SIGVTALRM    terminate process    virtual time alarm (see setitimer(2))
     SIGPROF      terminate process    profiling timer alarm (see
                                       setitimer(2))
     SIGWINCH     discard signal       window size change
     SIGINFO      discard signal       status request from keyboard
     SIGUSR1      terminate process    user-defined signal 1
     SIGUSR2      terminate process    user-defined signal 2
     SIGTHR       discard signal       thread AST
*/

#[cfg(target_os = "openbsd")]
pub static ALL_SIGNALS: [&str; 33] = [
    "EXIT", "HUP", "INT", "QUIT", "ILL", "TRAP", "ABRT", "EMT", "FPE", "KILL", "BUS", "SEGV",
    "SYS", "PIPE", "ALRM", "TERM", "URG", "STOP", "TSTP", "CONT", "CHLD", "TTIN", "TTOU", "IO",
    "XCPU", "XFSZ", "VTALRM", "PROF", "WINCH", "INFO", "USR1", "USR2", "THR",
];

/*
     The following signals are defined in Solaris and illumos;
     (the signals for illumos are the same as Solaris, but illumos still has SIGLWP
     as well as the alias for SIGLWP (SIGAIOCANCEL)):

     SIGHUP       1       hangup
     SIGINT       2       interrupt (rubout)
     SIGQUIT      3       quit (ASCII FS)
     SIGILL       4       illegal instruction (not reset when caught)
     SIGTRAP      5       trace trap (not reset when caught)
     SIGIOT       6       IOT instruction
     SIGABRT      6       used by abort, replace SIGIOT in the future
     SIGEMT       7       EMT instruction
     SIGFPE       8       floating point exception
     SIGKILL      9       kill (cannot be caught or ignored)
     SIGBUS       10      bus error
     SIGSEGV      11      segmentation violation
     SIGSYS       12      bad argument to system call
     SIGPIPE      13      write on a pipe with no one to read it
     SIGALRM      14      alarm clock
     SIGTERM      15      software termination signal from kill
     SIGUSR1      16      user defined signal 1
     SIGUSR2      17      user defined signal 2
     SIGCLD       18      child status change
     SIGCHLD      18      child status change alias (POSIX)
     SIGPWR       19      power-fail restart
     SIGWINCH     20      window size change
     SIGURG       21      urgent socket condition
     SIGPOLL      22      pollable event occurred
     SIGIO        SIGPOLL socket I/O possible (SIGPOLL alias)
     SIGSTOP      23      stop (cannot be caught or ignored)
     SIGTSTP      24      user stop requested from tty
     SIGCONT      25      stopped process has been continued
     SIGTTIN      26      background tty read attempted
     SIGTTOU      27      background tty write attempted
     SIGVTALRM    28      virtual timer expired
     SIGPROF      29      profiling timer expired
     SIGXCPU      30      exceeded cpu limit
     SIGXFSZ      31      exceeded file size limit
     SIGWAITING   32      reserved signal no longer used by threading code
     SIGAIOCANCEL 33      reserved signal no longer used by threading code (formerly SIGLWP)
     SIGFREEZE    34      special signal used by CPR
     SIGTHAW      35      special signal used by CPR
     SIGCANCEL    36      reserved signal for thread cancellation
     SIGLOST      37      resource lost (eg, record-lock lost)
     SIGXRES      38      resource control exceeded
     SIGJVM1      39      reserved signal for Java Virtual Machine
     SIGJVM2      40      reserved signal for Java Virtual Machine
     SIGINFO      41      information request
     SIGRTMIN     ((int)_sysconf(_SC_SIGRT_MIN)) first realtime signal
     SIGRTMAX     ((int)_sysconf(_SC_SIGRT_MAX)) last realtime signal
*/

#[cfg(target_os = "solaris")]
const SIGNALS_SIZE: usize = 46;

#[cfg(target_os = "illumos")]
const SIGNALS_SIZE: usize = 47;

#[cfg(any(target_os = "solaris", target_os = "illumos"))]
static ALL_SIGNALS: [&str; SIGNALS_SIZE] = [
    "HUP",
    "INT",
    "QUIT",
    "ILL",
    "TRAP",
    "IOT",
    "ABRT",
    "EMT",
    "FPE",
    "KILL",
    "BUS",
    "SEGV",
    "SYS",
    "PIPE",
    "ALRM",
    "TERM",
    "USR1",
    "USR2",
    "CLD",
    "CHLD",
    "PWR",
    "WINCH",
    "URG",
    "POLL",
    "IO",
    "STOP",
    "TSTP",
    "CONT",
    "TTIN",
    "TTOU",
    "VTALRM",
    "PROF",
    "XCPU",
    "XFSZ",
    "WAITING",
    "AIOCANCEL",
    #[cfg(target_os = "illumos")]
    "LWP",
    "FREEZE",
    "THAW",
    "CANCEL",
    "LOST",
    "XRES",
    "JVM1",
    "JVM2",
    "INFO",
    "RTMIN",
    "RTMAX",
];

/*
   The following signals are defined in AIX:

   SIGHUP     hangup, generated when terminal disconnects
   SIGINT     interrupt, generated from terminal special char
   SIGQUIT    quit, generated from terminal special char
   SIGILL     illegal instruction (not reset when caught)
   SIGTRAP    trace trap (not reset when caught)
   SIGABRT    abort process
   SIGEMT     EMT instruction
   SIGFPE     floating point exception
   SIGKILL    kill (cannot be caught or ignored)
   SIGBUS     bus error (specification exception)
   SIGSEGV    segmentation violation
   SIGSYS     bad argument to system call
   SIGPIPE    write on a pipe with no one to read it
   SIGALRM    alarm clock timeout
   SIGTERM    software termination signal
   SIGURG     urgent condition on I/O channel
   SIGSTOP    stop (cannot be caught or ignored)
   SIGTSTP    interactive stop
   SIGCONT    continue (cannot be caught or ignored)
   SIGCHLD    sent to parent on child stop or exit
   SIGTTIN    background read attempted from control terminal
   SIGTTOU    background write attempted to control terminal
   SIGIO      I/O possible, or completed
   SIGXCPU    cpu time limit exceeded (see setrlimit())
   SIGXFSZ    file size limit exceeded (see setrlimit())
   SIGMSG     input data is in the ring buffer
   SIGWINCH   window size changed
   SIGPWR     power-fail restart
   SIGUSR1    user defined signal 1
   SIGUSR2    user defined signal 2
   SIGPROF    profiling time alarm (see setitimer)
   SIGDANGER  system crash imminent; free up some page space
   SIGVTALRM  virtual time alarm (see setitimer)
   SIGMIGRATE migrate process
   SIGPRE     programming exception
   SIGVIRT    AIX virtual time alarm
   SIGTALRM   per-thread alarm clock
*/
#[cfg(target_os = "aix")]
pub static ALL_SIGNALS: [&str; 37] = [
    "HUP", "INT", "QUIT", "ILL", "TRAP", "ABRT", "EMT", "FPE", "KILL", "BUS", "SEGV", "SYS",
    "PIPE", "ALRM", "TERM", "URG", "STOP", "TSTP", "CONT", "CHLD", "TTIN", "TTOU", "IO", "XCPU",
    "XFSZ", "MSG", "WINCH", "PWR", "USR1", "USR2", "PROF", "DANGER", "VTALRM", "MIGRATE", "PRE",
    "VIRT", "TALRM",
];

/// Returns the signal number for a given signal name or value.
pub fn signal_by_name_or_value(signal_name_or_value: &str) -> Option<usize> {
    let signal_name_upcase = signal_name_or_value.to_uppercase();
    if let Ok(value) = signal_name_upcase.parse() {
        if is_signal(value) {
            return Some(value);
        } else {
            return None;
        }
    }
    let signal_name = signal_name_upcase.trim_start_matches("SIG");

    ALL_SIGNALS.iter().position(|&s| s == signal_name)
}

/// Returns true if the given number is a valid signal number.
pub fn is_signal(num: usize) -> bool {
    num < ALL_SIGNALS.len()
}

/// Returns the signal name for a given signal value.
pub fn signal_name_by_value(signal_value: usize) -> Option<&'static str> {
    ALL_SIGNALS.get(signal_value).copied()
}

/// Returns the default signal value.
#[cfg(unix)]
pub fn enable_pipe_errors() -> Result<(), Errno> {
    // We pass the error as is, the return value would just be Ok(SigDfl), so we can safely ignore it.
    // SAFETY: this function is safe as long as we do not use a custom SigHandler -- we use the default one.
    unsafe { signal(SIGPIPE, SigDfl) }.map(|_| ())
}

/// Ignores the SIGINT signal.
#[cfg(unix)]
pub fn ignore_interrupts() -> Result<(), Errno> {
    // We pass the error as is, the return value would just be Ok(SigIgn), so we can safely ignore it.
    // SAFETY: this function is safe as long as we do not use a custom SigHandler -- we use the default one.
    unsafe { signal(SIGINT, SigIgn) }.map(|_| ())
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
            signal_by_name_or_value(&format!("SIG{signal}")),
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
