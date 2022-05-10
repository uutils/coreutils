use crate::common::util::*;

// tests for basic tee functionality.
// inspired by:
// https://github.com/coreutils/coreutils/tests/misc/tee.sh

#[test]
fn test_tee_processing_multiple_operands() {
    // POSIX says: "Processing of at least 13 file operands shall be supported."

    let content = "tee_sample_content";
    for n in [1, 2, 12, 13] {
        let files = (1..=n).map(|x| x.to_string()).collect::<Vec<_>>();
        let (at, mut ucmd) = at_and_ucmd!();

        ucmd.args(&files)
            .pipe_in(content)
            .succeeds()
            .stdout_is(content);

        for file in &files {
            assert!(at.file_exists(file));
            assert_eq!(at.read(file), content);
        }
    }
}

#[test]
fn test_tee_treat_minus_as_filename() {
    // Ensure tee treats '-' as the name of a file, as mandated by POSIX.

    let (at, mut ucmd) = at_and_ucmd!();
    let content = "tee_sample_content";
    let file = "-";

    ucmd.arg("-").pipe_in(content).succeeds().stdout_is(content);

    assert!(at.file_exists(file));
    assert_eq!(at.read(file), content);
}

#[test]
fn test_tee_append() {
    let (at, mut ucmd) = at_and_ucmd!();
    let content = "tee_sample_content";
    let file = "tee_out";

    at.touch(file);
    at.write(file, content);
    assert_eq!(at.read(file), content);

    ucmd.arg("-a")
        .arg(file)
        .pipe_in(content)
        .succeeds()
        .stdout_is(content);
    assert!(at.file_exists(file));
    assert_eq!(at.read(file), content.repeat(2));
}

#[test]
#[cfg(target_os = "linux")]
fn test_tee_no_more_writeable_1() {
    // equals to 'tee /dev/full out2 <multi_read' call
    let (at, mut ucmd) = at_and_ucmd!();
    let content = (1..=10).map(|x| format!("{}\n", x)).collect::<String>();
    let file_out = "tee_file_out";

    ucmd.arg("/dev/full")
        .arg(file_out)
        .pipe_in(&content[..])
        .fails()
        .stdout_contains(&content)
        .stderr_contains(&"No space left on device");

    assert_eq!(at.read(file_out), content);
}

#[test]
#[cfg(target_os = "linux")]
fn test_tee_no_more_writeable_2() {
    // should be equals to 'tee out1 out2 >/dev/full <multi_read' call
    // but currently there is no way to redirect stdout to /dev/full
    // so this test is disabled
    let (_at, mut ucmd) = at_and_ucmd!();
    let _content = (1..=10).map(|x| format!("{}\n", x)).collect::<String>();
    let file_out_a = "tee_file_out_a";
    let file_out_b = "tee_file_out_b";

    let _result = ucmd
        .arg(file_out_a)
        .arg(file_out_b)
        .pipe_in("/dev/full")
        .succeeds(); // TODO: expected to succeed currently; change to fails() when required

    // TODO: comment in after https://github.com/uutils/coreutils/issues/1805 is fixed
    // assert_eq!(at.read(file_out_a), content);
    // assert_eq!(at.read(file_out_b), content);
    // assert!(result.stderr.contains("No space left on device"));
}
