// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (jargon) xattributes

#![cfg(feature = "feat_selinux")]

use uutests::util::*;

// TODO: Check the implementation of `--compute` somehow.

#[test]
fn version() {
    new_ucmd!().arg("--version").succeeds();
    new_ucmd!().arg("-V").succeeds();
}

#[test]
fn help() {
    new_ucmd!().arg("--help").succeeds();
    new_ucmd!().arg("-h").succeeds();
}

#[test]
fn invalid_input() {
    new_ucmd!().arg("-/").fails().code_is(125);
}

#[test]
fn print() {
    new_ucmd!().succeeds();

    for flag in ["-c", "--compute"] {
        new_ucmd!().arg(flag).succeeds();
    }

    for flag in [
        "-t", "--type", "-u", "--user", "-r", "--role", "-l", "--range",
    ] {
        new_ucmd!().args(&[flag, "example"]).succeeds();
        new_ucmd!().args(&[flag, "example1,example2"]).succeeds();
    }
}

#[test]
fn invalid() {
    new_ucmd!().arg("invalid").fails().code_is(1);

    let args = &[
        "unconfined_u:unconfined_r:unconfined_t:s0",
        "inexistent-file",
    ];
    new_ucmd!().args(args).fails().code_is(1);

    let args = &["invalid", "/bin/true"];
    new_ucmd!().args(args).fails().code_is(1);

    let args = &["--compute", "inexistent-file"];
    new_ucmd!().args(args).fails().code_is(1);

    let args = &["--compute", "--compute"];
    new_ucmd!().args(args).fails().code_is(125);

    // clap has an issue that makes this test fail: https://github.com/clap-rs/clap/issues/1543
    // TODO: Enable this code once the issue is fixed in the clap version we're using.
    //new_ucmd!().arg("--compute=example").fails().code_is(1);

    for flag in [
        "-t", "--type", "-u", "--user", "-r", "--role", "-l", "--range",
    ] {
        new_ucmd!().arg(flag).fails().code_is(125);

        let args = &[flag, "example", flag, "example"];
        new_ucmd!().args(args).fails().code_is(125);
    }
}

#[test]
#[cfg(feature = "feat_selinux")]
fn plain_context() {
    let ctx = "unconfined_u:unconfined_r:unconfined_t:s0-s0";
    new_ucmd!().args(&[ctx, "/bin/true"]).succeeds();
    new_ucmd!().args(&[ctx, "/bin/false"]).fails().code_is(1);

    let output = new_ucmd!().args(&[ctx, "sestatus", "-v"]).succeeds();
    let r = get_sestatus_context(output.stdout());
    assert_eq!(r, "unconfined_u:unconfined_r:unconfined_t:s0");

    let ctx = "system_u:unconfined_r:unconfined_t:s0-s0";
    new_ucmd!().args(&[ctx, "/bin/true"]).succeeds();

    let ctx = "system_u:system_r:unconfined_t:s0";
    let output = new_ucmd!().args(&[ctx, "sestatus", "-v"]).succeeds();
    assert_eq!(get_sestatus_context(output.stdout()), ctx);
}

#[test]
#[cfg(feature = "feat_selinux")]
fn custom_context() {
    let t_ud = "unconfined_t";
    let u_ud = "unconfined_u";
    let r_ud = "unconfined_r";

    new_ucmd!().args(&["--compute", "/bin/true"]).succeeds();

    let args = &["--compute", "/bin/false"];
    new_ucmd!().args(args).fails().code_is(1);

    let args = &["--type", t_ud, "/bin/true"];
    new_ucmd!().args(args).succeeds();

    let args = &["--compute", "--type", t_ud, "/bin/true"];
    new_ucmd!().args(args).succeeds();

    let args = &["--user=system_u", "/bin/true"];
    new_ucmd!().args(args).succeeds();

    let args = &["--compute", "--user=system_u", "/bin/true"];
    new_ucmd!().args(args).succeeds();

    let args = &["--role=system_r", "/bin/true"];
    new_ucmd!().args(args).succeeds();

    let args = &["--compute", "--role=system_r", "/bin/true"];
    new_ucmd!().args(args).succeeds();

    new_ucmd!().args(&["--range=s0", "/bin/true"]).succeeds();

    let args = &["--compute", "--range=s0", "/bin/true"];
    new_ucmd!().args(args).succeeds();

    for (ctx, u, r) in [
        ("unconfined_u:unconfined_r:unconfined_t:s0", u_ud, r_ud),
        ("system_u:unconfined_r:unconfined_t:s0", "system_u", r_ud),
        ("unconfined_u:system_r:unconfined_t:s0", u_ud, "system_r"),
        ("system_u:system_r:unconfined_t:s0", "system_u", "system_r"),
    ] {
        let args = &["-t", t_ud, "-u", u, "-r", r, "-l", "s0", "sestatus", "-v"];

        let output = new_ucmd!().args(args).succeeds();
        assert_eq!(get_sestatus_context(output.stdout()), ctx);
    }
}

fn get_sestatus_context(output: &[u8]) -> &str {
    let re = regex::bytes::Regex::new(r"Current context:\s*(\S+)\s*")
        .expect("Invalid regular expression");

    output
        .split(|&b| b == b'\n')
        .find(|&b| b.starts_with(b"Current context:"))
        .and_then(|line| {
            re.captures_iter(line)
                .next()
                .and_then(|c| c.get(1))
                .as_ref()
                .map(regex::bytes::Match::as_bytes)
        })
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .expect("Output of sestatus is unexpected")
}
