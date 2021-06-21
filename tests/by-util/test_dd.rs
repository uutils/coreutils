use crate::common::util::*;

#[test]
fn version()
{
    new_ucmd!()
        .args(&["--version"])
        .succeeds();
}

#[test]
fn help()
{
    new_ucmd!()
        .args(&["--help"])
        .succeeds();
}

fn build_ascii_block(n: usize) -> Vec<u8>
{
    vec!['a', 'b', 'c', 'd', 'e', 'f']
        .into_iter()
        .map(|c| c as u8)
        .cycle()
        .take(n)
        .collect()
}

#[test]
fn test_stdin_stdout()
{
    let input = build_ascii_block(521);
    let output = String::from_utf8(input.clone()).unwrap();
    new_ucmd!()
        .args(&["status=none"])
        .pipe_in(input)
        .succeeds()
        .stdout_only(output);
}

#[test]
fn test_stdin_stdout_count()
{
    let input = build_ascii_block(521);
    let mut output = String::from_utf8(input.clone()).unwrap();
    output.truncate(256);
    new_ucmd!()
        .args(&[
            "status=none",
            "count=2",
            "ibs=128",
        ])
        .pipe_in(input)
        .succeeds()
        .stdout_only(output);
}

#[test]
fn test_stdin_stdout_count_bytes()
{
    let input = build_ascii_block(521);
    let mut output = String::from_utf8(input.clone()).unwrap();
    output.truncate(256);
    new_ucmd!()
        .args(&[
            "status=none",
            "count=256",
            "iflag=count_bytes",
        ])
        .pipe_in(input)
        .succeeds()
        .stdout_only(output);
}

#[test]
fn test_stdin_stdout_skip()
{
    let input = build_ascii_block(521);
    let mut output = String::from_utf8(input.clone()).unwrap();
    let _ = output.drain(..256);
    new_ucmd!()
        .args(&[
            "status=none",
            "skip=2",
            "ibs=128",
        ])
        .pipe_in(input)
        .succeeds()
        .stdout_only(output);
}

#[test]
fn test_stdin_stdout_skip_bytes()
{
    let input = build_ascii_block(521);
    let mut output = String::from_utf8(input.clone()).unwrap();
    let _ = output.drain(..256);
    new_ucmd!()
        .args(&[
            "status=none",
            "skip=256",
            "ibs=128",
            "iflag=skip_bytes",
        ])
        .pipe_in(input)
        .succeeds()
        .stdout_only(output);
}

#[test]
fn test_final_stats_noxfer()
{
   new_ucmd!()
        .args(&[
            "status=noxfer",
        ])
        .succeeds()
        .stderr_only("");
}

#[test]
fn test_final_stats_unspec()
{
    let output = vec![
        "0+0 records in",
        "0+0 records out",
        "0 bytes (0 B, 0 B) copied, 0.0 s, 0 B/s",
    ];
    let output = output.into_iter()
                       .fold(String::new(), | mut acc, s | {
                           acc.push_str(s);
                           acc.push('\n');
                           acc
                       });
    new_ucmd!()
        .succeeds()
        .stderr_only(&output);
}

#[test]
fn test_self_transfer()
{
    panic!();
    // TODO: Make new copy per-test
    new_ucmd!()
        .args(&[
            "conv=notruc",
            "if=../fixtures/dd/zero-256k.copy",
            "of=../fixtures/dd/zero-256k.copy",
        ])
        .succeeds();
    assert!(false/* Must check that zero256k.copy still == zero-256k.txt */)
}

#[cfg(unix)]
#[test]
fn test_null()
{
    let stats = vec![
        "0+0 records in",
        "0+0 records out",
        "0 bytes (0 B, 0 B) copied, 0.0 s, 0 B/s",
    ];
    let stats = stats.into_iter()
                       .fold(String::new(), | mut acc, s | {
                           acc.push_str(s);
                           acc.push('\n');
                           acc
                       });
    new_ucmd!()
        .args(&[
            "if=/dev/null",
        ])
        .succeeds()
        .stderr_only(stats)
        .stdout_only("");
}

#[test]
fn test_ys_to_stdout()
{
    let output: Vec<_> = String::from("y\n")
        .bytes()
        .cycle()
        .take(1024)
        .collect();
    let output = String::from_utf8(output).unwrap();

    new_ucmd!()
        .args(&[
            "if=../fixtures/dd/y-nl-1k.txt",
        ])
        .run()
        .stdout_only(output);
}

#[test]
fn test_zeros_to_stdout()
{
    let output = vec![0; 256*1024];
    let output = String::from_utf8(output).unwrap();
    new_ucmd!()
        .args(&[
            "if=../fixtures/dd/zero-256k.txt",
        ])
        .run()
        .stdout_only(output);
}

