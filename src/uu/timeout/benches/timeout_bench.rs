// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_timeout::uumain;
use uucore::benchmark::get_bench_args;

/// First-arg marker that re-runs this bench binary as its own child command,
/// a portable `sleep`/`true` stand-in (`cargo bench` builds no other binary
/// and Windows lacks both in `PATH`).
const CHILD_MARKER: &str = "__timeout_bench_child";

fn self_exe() -> String {
    std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

/// Benchmark the fast path where the command exits immediately.
#[cfg(unix)]
#[divan::bench]
fn timeout_quick_exit(bencher: Bencher) {
    let args = get_bench_args(&[&"0.02", &"true"]);
    bencher
        .with_inputs(|| args.clone().into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark the fast path where the command exits immediately.
#[cfg(windows)]
#[divan::bench]
fn timeout_quick_exit(bencher: Bencher) {
    let exe = self_exe();
    let args = get_bench_args(&[&"0.02", &exe, &CHILD_MARKER, &"0"]);
    bencher
        .with_inputs(|| args.clone().into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark a command that runs longer than the threshold and receives the default signal.
#[cfg(unix)]
#[divan::bench]
fn timeout_enforced(bencher: Bencher) {
    let args = get_bench_args(&[&"0.02", &"sleep", &"0.2"]);
    bencher
        .with_inputs(|| args.clone().into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Benchmark a command that runs longer than the threshold: timer expiry,
/// job-object tree kill and exit-code plumbing.
#[cfg(windows)]
#[divan::bench]
fn timeout_enforced(bencher: Bencher) {
    let exe = self_exe();
    let args = get_bench_args(&[&"0.02", &exe, &CHILD_MARKER, &"0.2"]);
    bencher
        .with_inputs(|| args.clone().into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

/// Track timeout-firing latency across small durations; a regression from the
/// high-resolution timer (100 ns waitable timer on Windows, POSIX interval
/// timers on unix) to a coarser wait shows up as a step change across the
/// arguments.
#[divan::bench(args = ["0.001", "0.005", "0.02"])]
fn timer_expiry_latency(bencher: Bencher, duration: &str) {
    let exe = self_exe();
    let args = get_bench_args(&[&duration, &exe, &CHILD_MARKER, &"5"]);
    bencher
        .with_inputs(|| args.clone().into_iter())
        .bench_values(|args| black_box(uumain(args)));
}

fn main() {
    // When re-invoked with CHILD_MARKER, act as a tiny `sleep` replacement.
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some(CHILD_MARKER) {
        if let Some(secs) = args.next().and_then(|s| s.parse::<f64>().ok())
            && secs > 0.0
        {
            std::thread::sleep(std::time::Duration::from_secs_f64(secs));
        }
        return;
    }
    divan::main();
}
