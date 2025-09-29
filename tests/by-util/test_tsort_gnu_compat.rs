// This file is part of the uutils coreutils package.
// Linux-only GNU tsort compatibility checks.
// We compare uutils tsort against the system tsort (GNU on Linux) on a small corpus.

#![cfg(target_os = "linux")]

use std::process::{Command, Stdio};

use uutests::new_ucmd;

fn run_gnu_tsort(input: &str) -> (String, String) {
    let mut child = Command::new("tsort")
        .arg("-")
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn GNU tsort");

    {
        use std::io::Write;
        let mut stdin = child.stdin.take().expect("stdin pipe");
        stdin
            .write_all(input.as_bytes())
            .expect("write input to GNU tsort");
    }

    let out = child.wait_with_output().expect("wait for GNU tsort");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

fn run_uutils_tsort(input: &str) -> (String, String) {
    let mut cmd = new_ucmd!();
    let res = cmd.env("LANG", "C").env("LC_ALL", "C").arg("-").pipe_in(input).run();
    (res.stdout_str(), res.stderr_str())
}

fn normalize(s: &str) -> String {
    // Trim trailing whitespace to reduce noise; keep line boundaries.
    s.lines().map(|l| l.trim_end()).collect::<Vec<_>>().join("\n") + "\n"
}

fn case(name: &str, input: &str) {
    let (gnu_out, gnu_err) = run_gnu_tsort(input);
    let (uu_out, uu_err) = run_uutils_tsort(input);

    let gnu_out = normalize(&gnu_out);
    let gnu_err = normalize(&gnu_err);
    let uu_out = normalize(&uu_out);
    let uu_err = normalize(&uu_err);

    assert_eq!(uu_err, gnu_err, "stderr mismatch in case '{}':\nGNU:\n{}\nUU:\n{}", name, gnu_err, uu_err);
    assert_eq!(uu_out, gnu_out, "stdout mismatch in case '{}':\nGNU:\n{}\nUU:\n{}", name, gnu_out, uu_out);
}

#[test]
fn gnu_compat_issue_sample() {
    // From #8743
    let input = "A B\nB C\nC B\nC D\nD A\n";
    case("issue_sample_two_cycles", input);
}

#[test]
fn gnu_compat_small_cycle_with_path() {
    // a -> b <-> c -> d
    let input = "a b\nb c\nc d\nc b\n";
    case("small_cycle_with_path", input);
}

#[test]
fn gnu_compat_duplicates() {
    // Duplicate edges b->c
    let input = "a b\nb c\nb c\nc b\n";
    case("duplicates", input);
}

#[test]
fn gnu_compat_disjoint_cycles_and_chain() {
    // A<->B, X<->Y, and M->N
    let input = "A B\nB A\nX Y\nY X\nM N\n";
    case("disjoint_cycles_and_chain", input);
}



#[test]
fn gnu_compat_self_loop() {
    // Self-loop: validate GNU behavior (reports cycle or ignores). Linux-only.
    let input = "X X\n";
    case("self_loop", input);
}

#[test]
fn gnu_compat_multiple_sccs_shared_nodes() {
    // Two SCCs sharing a node via a bridge: A<->B, C<->D, and B->C
    let input = "A B\nB A\nC D\nD C\nB C\n";
    case("multiple_sccs_shared_nodes", input);
}

#[test]
fn gnu_compat_varied_insertion_orders() {
    // Same logical graph, different insertion order to ensure ordering robustness
    let a = "A B\nB C\nC A\n"; // order 1
    let b = "B C\nC A\nA B\n"; // order 2
    case("varied_order_a", a);
    case("varied_order_b", b);
}

#[test]
fn gnu_compat_shared_node_across_sccs() {
    // A<->B, B->C, C<->D (B bridges into next SCC)
    let input = "A B\nB A\nB C\nC D\nD C\n";
    case("shared_node_across_sccs", input);
}

#[test]
fn gnu_compat_large_cycle() {
    // Moderate large cycle to keep CI fast but still validate behavior on larger cases.
    let n = 200usize;
    let mut s = String::new();
    for i in 1..=n {
        let j = if i == n { 1 } else { i + 1 };
        s.push_str(&format!("{} {}\n", i, j));
    }
    case("large_cycle_200", &s);
}
