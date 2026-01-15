// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::env;
use std::process;
use std::time::Duration;

const CHILD_FLAG: &str = "--timeout-bench-child";

fn maybe_run_child_mode() {
    let mut args = env::args();
    let _ = args.next(); // skip executable path

    while let Some(arg) = args.next() {
        if arg == CHILD_FLAG {
            let mode = args
                .next()
                .unwrap_or_else(|| panic!("missing child mode after {CHILD_FLAG}"));
            run_child(mode);
        }
    }
}

#[cfg(unix)]
fn run_child(mode: String) -> ! {
    match mode.as_str() {
        "quick-exit" => process::exit(0),
        "short-sleep" => {
            std::thread::sleep(Duration::from_millis(5));
            process::exit(0);
        }
        "long-sleep" => {
            std::thread::sleep(Duration::from_millis(200));
            process::exit(0);
        }
        "ignore-term" => {
            use nix::sys::signal::{SigHandler, Signal, signal};

            unsafe {
                signal(Signal::SIGTERM, SigHandler::SigIgn)
                    .expect("failed to ignore SIGTERM in bench child");
            }

            loop {
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        other => {
            eprintln!("unknown child mode: {other}");
            process::exit(1);
        }
    }
}

#[cfg(not(unix))]
fn run_child(_: String) -> ! {
    // The timeout benchmarks are Unix-only, but ensure child invocations still terminate.
    process::exit(0);
}

#[cfg(unix)]
mod unix {
    use super::*;
    use divan::{Bencher, black_box};
    use uu_timeout::uumain;
    use uucore::benchmark::run_util_function;

    fn bench_timeout_with_mode(bencher: Bencher, args: &[&str], child_mode: &str) {
        let child_path = env::current_exe()
            .expect("failed to locate timeout bench executable")
            .into_os_string()
            .into_string()
            .expect("bench executable path must be valid UTF-8");

        let mut owned_args: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
        owned_args.push(child_path);
        owned_args.push(CHILD_FLAG.into());
        owned_args.push(child_mode.to_string());

        let arg_refs: Vec<&str> = owned_args.iter().map(|s| s.as_str()).collect();

        bencher.bench(|| {
            black_box(run_util_function(uumain, &arg_refs));
        });
    }

    /// Benchmark the fast path where the command exits immediately.
    #[divan::bench]
    fn timeout_quick_exit(bencher: Bencher) {
        bench_timeout_with_mode(bencher, &["0.02"], "quick-exit");
    }

    /// Benchmark a command that runs longer than the threshold and receives the default signal.
    #[divan::bench]
    fn timeout_enforced(bencher: Bencher) {
        bench_timeout_with_mode(bencher, &["0.02"], "long-sleep");
    }

    pub fn run() {
        divan::main();
    }
}

#[cfg(unix)]
fn main() {
    maybe_run_child_mode();
    unix::run();
}

#[cfg(not(unix))]
fn main() {
    maybe_run_child_mode();
}
