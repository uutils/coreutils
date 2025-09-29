// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
#![allow(clippy::cast_possible_wrap)]

use uutests::at_and_ucmd;
use uutests::new_ucmd;

#[test]
#[cfg(target_os = "linux")]
fn test_tsort_non_utf8_paths() {
    use std::os::unix::ffi::OsStringExt;
    let (at, mut ucmd) = at_and_ucmd!();

    let filename = std::ffi::OsString::from_vec(vec![0xFF, 0xFE]);
    std::fs::write(at.plus(&filename), b"a b\nb c\n").unwrap();

    ucmd.arg(&filename).succeeds().stdout_is("a\nb\nc\n");
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}
#[test]
fn test_sort_call_graph() {
    new_ucmd!()
        .arg("call_graph.txt")
        .succeeds()
        .stdout_is_fixture("call_graph.expected");
}

#[test]
fn test_sort_self_loop() {
    new_ucmd!()
        .pipe_in("first first\nfirst second second second")
        .succeeds()
        .stdout_only("first\nsecond\n");
}

#[test]
fn test_sort_floating_nodes() {
    new_ucmd!()
        .pipe_in("d d\nc c\na a\nb b")
        .succeeds()
        .stdout_only("a\nb\nc\nd\n");
}

#[test]
fn test_no_such_file() {
    new_ucmd!()
        .arg("invalid_file_txt")
        .fails()
        .stderr_contains("No such file or directory");
}

#[test]
fn test_version_flag() {
    let version_short = new_ucmd!().arg("-V").succeeds();
    let version_long = new_ucmd!().arg("--version").succeeds();

    assert_eq!(version_short.stdout_str(), version_long.stdout_str());
}

#[test]
fn test_help_flag() {
    let help_short = new_ucmd!().arg("-h").succeeds();
    let help_long = new_ucmd!().arg("--help").succeeds();

    assert_eq!(help_short.stdout_str(), help_long.stdout_str());
}

#[test]
fn test_multiple_arguments() {
    new_ucmd!()
        .arg("call_graph.txt")
        .arg("invalid_file")
        .fails()
        .stderr_contains("unexpected argument 'invalid_file' found");
}

#[test]
fn test_error_on_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("tsort_test_dir");
    ucmd.arg("tsort_test_dir")
        .fails()
        .stderr_contains("tsort: tsort_test_dir: read error: Is a directory");
}

#[test]
fn test_split_on_any_whitespace() {
    new_ucmd!()
        .pipe_in("a\nb\n")
        .succeeds()
        .stdout_only("a\nb\n");
}

#[test]
fn test_cycle() {
    // The graph looks like:  a --> b <==> c --> d
    let res = new_ucmd!().pipe_in("a b b c c d c b").fails_with_code(1);

    // stderr: single cycle [b, c] (order may vary but nodes must be exactly b and c)
    let stderr = res.stderr_str();
    assert!(stderr.starts_with("tsort: -: input contains a loop:\n"));
    let cycle_lines: Vec<&str> = stderr.lines().skip(1).collect();
    assert_eq!(
        cycle_lines.len(),
        2,
        "expected exactly two cycle nodes in stderr, got: {stderr}",
    );
    let mut nodes = cycle_lines.clone();
    nodes.sort_unstable();
    assert_eq!(nodes, vec!["tsort: b", "tsort: c"]);

    // stdout: all nodes exactly once in any order
    let stdout = res.stdout_str();
    let mut out_nodes: Vec<&str> = stdout.lines().collect();
    out_nodes.sort_unstable();
    assert_eq!(out_nodes, vec!["a", "b", "c", "d"]);
}

#[test]
fn test_two_cycles() {
    // The graph looks like:
    //        a -> b <-> c and b <-> d (two cycles sharing b)
    let res = new_ucmd!()
        .pipe_in("a b b c c b b d d b")
        .fails_with_code(1);

    // stderr should report two loops. Collect them by splitting blocks.
    let stderr = res.stderr_str();
    let lines: Vec<&str> = stderr.lines().collect();
    // Expected structure:
    // tsort: -: input contains a loop:
    // tsort: X
    // tsort: Y
    // tsort: -: input contains a loop:
    // tsort: X
    // tsort: Y
    assert!(lines.len() >= 6, "unexpected stderr: {stderr}");
    assert_eq!(lines[0], "tsort: -: input contains a loop:");
    let mut first_cycle = vec![lines[1], lines[2]];
    first_cycle.sort_unstable();
    assert_eq!(
        first_cycle,
        vec!["tsort: b", "tsort: c"],
        "first cycle should be b,c: {stderr}",
    );

    assert_eq!(lines[3], "tsort: -: input contains a loop:");
    let mut second_cycle = vec![lines[4], lines[5]];
    second_cycle.sort_unstable();
    assert_eq!(
        second_cycle,
        vec!["tsort: b", "tsort: d"],
        "second cycle should be b,d: {stderr}",
    );

    // stdout: all nodes exactly once in any order
    let stdout = res.stdout_str();
    let mut out_nodes: Vec<&str> = stdout.lines().collect();
    out_nodes.sort_unstable();
    assert_eq!(out_nodes, vec!["a", "b", "c", "d"]);
}

#[test]
fn test_duplicate_edges_cycle_and_order() {
    // Duplicate edges should not break correctness; cycle detection remains correct.
    // Graph: a -> b, b -> c, c -> b (cycle b<->c), with duplicates of b->c
    let input = "a b\nb c\nb c\nb c\nc b\n";
    let res = new_ucmd!().pipe_in(input).fails_with_code(1);

    // stderr: one loop with nodes {b,c}
    let stderr = res.stderr_str();
    let lines: Vec<&str> = stderr.lines().collect();
    assert!(lines.len() >= 3, "unexpected stderr: {stderr}");
    assert_eq!(lines[0], "tsort: -: input contains a loop:");
    let mut cyc = vec![lines[1], lines[2]];
    cyc.sort_unstable();
    assert_eq!(
        cyc,
        vec!["tsort: b", "tsort: c"],
        "expected cycle b,c: {stderr}",
    );

    // stdout contains all distinct nodes exactly once
    let stdout = res.stdout_str();
    let mut out_nodes: Vec<&str> = stdout.lines().collect();
    out_nodes.sort_unstable();
    assert_eq!(out_nodes, vec!["a", "b", "c"]);
}

#[test]
fn test_mixed_cycles_and_chain_determinism() {
    // Disjoint cycles plus independent chain should be handled deterministically.
    // Graph: A<->B, X<->Y, and M->N->O
    let input = "A B\nB A\nX Y\nY X\nM N\nN O\n";
    let res = new_ucmd!().pipe_in(input).fails_with_code(1);

    // Expect two cycles reported (A,B) and (X,Y) in some deterministic order.
    let stderr = res.stderr_str();
    let blocks: Vec<&str> = stderr
        .split("tsort: -: input contains a loop:\n")
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(blocks.len(), 2, "expected two cycle reports, got: {stderr}",);
    for block in &blocks {
        let mut nodes: Vec<&str> = block.lines().take(2).collect();
        nodes.sort_unstable();
        assert!(
            nodes == vec!["tsort: A", "tsort: B"] || nodes == vec!["tsort: X", "tsort: Y"],
            "unexpected cycle nodes: {block}",
        );
    }

    // stdout: all nodes exactly once
    let stdout = res.stdout_str();
    let mut out_nodes: Vec<&str> = stdout.lines().collect();
    out_nodes.sort_unstable();
    assert_eq!(out_nodes, vec!["A", "B", "M", "N", "O", "X", "Y"]);
}

#[test]
fn test_moderate_large_cycle() {
    // Construct a moderate cycle to avoid stack overflow while documenting behavior.
    // Note: very large cycles can overflow the recursive DFS (see #8695). This test keeps it moderate.
    let n = 300usize;
    let mut input = String::new();
    for i in 1..=n {
        let j = if i == n { 1 } else { i + 1 };
        input.push_str(&i.to_string());
        input.push(' ');
        input.push_str(&j.to_string());
        input.push('\n');
    }
    let res = new_ucmd!().pipe_in(input).fails_with_code(1);

    // Should report at least one cycle header and two nodes (exact nodes depend on traversal)
    let stderr = res.stderr_str();
    assert!(
        stderr.starts_with("tsort: -: input contains a loop:\n"),
        "stderr: {stderr}",
    );
    assert!(stderr.lines().count() >= 3, "too short stderr: {stderr}");
}

#[test]
fn test_issue_8743_two_cycles_reporting() {
    // Input from issue #8743:
    // A B
    // B C
    // C B
    // C D
    // D A
    // Expect two loop reports: first the minimal cycle [B, C], then the larger cycle [A, B, C, D].
    new_ucmd!()
        .pipe_in("A B\nB C\nC B\nC D\nD A\n")
        .fails_with_code(1)
        .stderr_is(
            "tsort: -: input contains a loop:\n".to_string()
                + "tsort: B\n"
                + "tsort: C\n"
                + "tsort: -: input contains a loop:\n"
                + "tsort: A\n"
                + "tsort: B\n"
                + "tsort: C\n"
                + "tsort: D\n",
        );
}
