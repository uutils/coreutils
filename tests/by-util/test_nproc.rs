// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore incorrectnumber

use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_nproc() {
    let nproc: u8 = new_ucmd!().succeeds().stdout_str().trim().parse().unwrap();
    assert!(nproc > 0);
}

#[test]
fn test_nproc_all_omp() {
    let result = new_ucmd!().arg("--all").succeeds();

    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert!(nproc > 0);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "60")
        .succeeds();

    let nproc_omp: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc_omp, 60);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "1") // Has no effect
        .arg("--all")
        .succeeds();
    let nproc_omp: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, nproc_omp);

    // If the parsing fails, returns the number of CPU
    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "incorrectnumber") // returns the number CPU
        .succeeds();
    let nproc_omp: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, nproc_omp);

    // clamp overflow
    // The wasm guest is always a 32-bit target regardless of the host's
    // pointer width, so it clamps to u32::MAX there instead of u64::MAX.
    #[cfg(any(target_pointer_width = "64", wasi_runner))]
    TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "99999999999999999999")
        .succeeds()
        .stdout_only(if cfg!(wasi_runner) {
            "4294967295\n"
        } else {
            "18446744073709551615\n"
        });
}

#[test]
fn test_nproc_ignore() {
    let result = new_ucmd!().succeeds();
    let nproc_total: u8 = result.stdout_str().trim().parse().unwrap();
    if nproc_total > 1 {
        // Ignore all CPU but one
        let result = TestScenario::new(util_name!())
            .ucmd()
            .arg("--ignore")
            .arg((nproc_total - 1).to_string())
            .succeeds();
        let nproc: u8 = result.stdout_str().trim().parse().unwrap();
        assert_eq!(nproc, 1);
        // Ignore all CPU but one with a string
        let result = TestScenario::new(util_name!())
            .ucmd()
            .arg("--ignore= 1")
            .succeeds();
        let nproc: u8 = result.stdout_str().trim().parse().unwrap();
        assert_eq!(nproc_total - 1, nproc);
        // overflow
        TestScenario::new(util_name!())
            .ucmd()
            .arg("--ignore=99999999999999999999")
            .succeeds()
            .stdout_only("1\n");
    }
}

#[test]
fn test_nproc_ignore_all_omp() {
    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "42")
        .arg("--ignore=40")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, 2);
}

#[test]
fn test_nproc_omp_limit() {
    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "42")
        .env("OMP_THREAD_LIMIT", "0")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, 42);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "42")
        .env("OMP_THREAD_LIMIT", "2")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, 2);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "42")
        .env("OMP_THREAD_LIMIT", "2bad")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, 42);

    let result = new_ucmd!().arg("--all").succeeds();
    let nproc_system: u8 = result.stdout_str().trim().parse().unwrap();
    assert!(nproc_system > 0);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_THREAD_LIMIT", "1")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, 1);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "0")
        .env("OMP_THREAD_LIMIT", "")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, nproc_system);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "")
        .env("OMP_THREAD_LIMIT", "")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, nproc_system);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "2,2,1")
        .env("OMP_THREAD_LIMIT", "")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(2, nproc);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "2,ignored")
        .env("OMP_THREAD_LIMIT", "")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(2, nproc);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "2,2,1")
        .env("OMP_THREAD_LIMIT", "0")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(2, nproc);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "2,2,1")
        .env("OMP_THREAD_LIMIT", "1bad")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(2, nproc);

    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", "29,2,1")
        .env("OMP_THREAD_LIMIT", "1bad")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(29, nproc);
}

#[test]
fn test_nproc_omp_num_threads_with_whitespace() {
    // OMP_NUM_THREADS with leading/trailing spaces should be trimmed and parsed correctly.
    // Before the fix, " 42 " would fail to parse and fall back to available_parallelism().
    let result = TestScenario::new(util_name!())
        .ucmd()
        .env("OMP_NUM_THREADS", " 42 ")
        .succeeds();
    let nproc: u8 = result.stdout_str().trim().parse().unwrap();
    assert_eq!(nproc, 42);
}

/// Tests that run `nproc` in a transient systemd scope with CPU quotas.
///
/// They need a systemd user manager that can delegate the cpu controller
/// (`systemd-run --user` on a desktop session or GitHub's Ubuntu runners),
/// and are skipped otherwise.
#[cfg(all(target_os = "linux", not(wasi_runner)))]
mod cgroup_quota {
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use uutests::util::TestScenario;
    use uutests::util_name;

    /// Moves the shell into a `leaf` child of the delegated scope, optionally
    /// sets a quota of `$1` CPUs there, and runs the rest of the arguments.
    /// systemd removes the scope and its children when the command exits.
    const SCRIPT: &str = r#"cg=/sys/fs/cgroup$(sed -n 's/^0:://p' /proc/self/cgroup) &&
mkdir "$cg/leaf" && echo 0 > "$cg/leaf/cgroup.procs" &&
echo +cpu > "$cg/cgroup.subtree_control" &&
{ [ -z "$1" ] || echo "$(($1 * 100000)) 100000" > "$cg/leaf/cpu.max"; } &&
shift && exec "$@""#;

    /// Run `nproc` in a delegated scope named `unit` with a `CPUQuota` of
    /// `scope_quota` and a quota of `leaf_quota` CPUs on its `leaf` child.
    /// Returns `None` if the scope cannot be set up here (no user manager,
    /// no cpu delegation, broken D-Bus session, ...).
    fn nproc_in_scope(
        unit: &str,
        scope_quota: Option<&str>,
        leaf_quota: Option<u32>,
    ) -> Option<usize> {
        // systemd may still be collecting a scope that just exited, so never
        // reuse a unit name.
        static RUN: AtomicUsize = AtomicUsize::new(0);
        let run = RUN.fetch_add(1, Ordering::Relaxed);
        let ts = TestScenario::new(util_name!());
        let mut command = Command::new("systemd-run");
        command
            .args(["--user", "--quiet", "--scope", "--collect"])
            .arg("--expand-environment=no")
            .arg(format!(
                "--unit=uu-nproc-{unit}-{}-{run}",
                std::process::id()
            ))
            .args(["-p", "Delegate=yes"]);
        if let Some(quota) = scope_quota {
            command.arg("-p").arg(format!("CPUQuota={quota}"));
        }
        let output = command
            .args(["sh", "-c", SCRIPT, "sh"])
            .arg(leaf_quota.map(|q| q.to_string()).unwrap_or_default())
            .arg(&ts.bin_path)
            .arg(util_name!())
            .env_remove("OMP_NUM_THREADS")
            .env_remove("OMP_THREAD_LIMIT")
            .output();
        match output {
            Ok(output) if output.status.success() => Some(
                String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .parse()
                    .unwrap(),
            ),
            Ok(output) => {
                println!(
                    "test skipped: systemd-run --user failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                );
                None
            }
            Err(e) => {
                println!("test skipped: cannot run systemd-run: {e}");
                None
            }
        }
    }

    fn unlimited() -> usize {
        TestScenario::new(util_name!())
            .ucmd()
            .succeeds()
            .stdout_str()
            .trim()
            .parse()
            .unwrap()
    }

    #[test]
    fn test_nproc_cgroup_quota() {
        if let Some(n) = nproc_in_scope("leaf", None, Some(1)) {
            assert_eq!(n, unlimited().min(1));
        }
    }

    #[test]
    fn test_nproc_cgroup_quota_path_with_colon() {
        if let Some(n) = nproc_in_scope("a:b", None, Some(1)) {
            assert_eq!(n, unlimited().min(1));
        }
    }

    #[test]
    fn test_nproc_cgroup_quota_on_parent() {
        if let Some(n) = nproc_in_scope("parent", Some("149%"), None) {
            assert_eq!(n, unlimited().min(1));
        }
        if let Some(n) = nproc_in_scope("parent", Some("151%"), None) {
            assert_eq!(n, unlimited().min(2));
        }
    }

    #[test]
    fn test_nproc_cgroup_quota_smallest_wins() {
        if let Some(n) = nproc_in_scope("small-parent", Some("100%"), Some(3)) {
            assert_eq!(n, unlimited().min(1));
        }
        if let Some(n) = nproc_in_scope("small-leaf", Some("300%"), Some(1)) {
            assert_eq!(n, unlimited().min(1));
        }
    }
}
