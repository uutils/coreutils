use crate::common::util::*;

#[test]
fn test_df_compatible_no_size_arg() {
    new_ucmd!().arg("-a").succeeds();
}

#[test]
fn test_df_shortened_long_argument() {
    new_ucmd!().arg("--a").succeeds();
}

#[test]
fn test_df_compatible() {
    new_ucmd!().arg("-ah").succeeds();
}

#[test]
fn test_df_compatible_type() {
    new_ucmd!().arg("-aT").succeeds();
}

#[test]
fn test_df_compatible_si() {
    new_ucmd!().arg("-aH").succeeds();
}

#[test]
fn test_df_output() {
    if cfg!(target_os = "macos") {
        new_ucmd!().arg("-H").arg("-total").succeeds().
        stdout_only("Filesystem               Size         Used    Available     Capacity  Use% Mounted on       \n");
    } else {
        new_ucmd!().arg("-H").arg("-total").succeeds().stdout_only(
            "Filesystem               Size         Used    Available  Use% Mounted on       \n",
        );
    }
}

/// Test that the order of rows in the table does not change across executions.
#[test]
fn test_order_same() {
    // TODO When #3057 is resolved, we should just use
    //
    //     new_ucmd!().arg("--output=source").succeeds().stdout_move_str();
    //
    // instead of parsing the entire `df` table as a string.
    let output1 = new_ucmd!().succeeds().stdout_move_str();
    let output2 = new_ucmd!().succeeds().stdout_move_str();
    let output1: Vec<String> = output1
        .lines()
        .map(|l| String::from(l.split_once(' ').unwrap().0))
        .collect();
    let output2: Vec<String> = output2
        .lines()
        .map(|l| String::from(l.split_once(' ').unwrap().0))
        .collect();
    assert_eq!(output1, output2);
}

/// Test of mount point begin repeated
#[cfg(unix)]
#[test]
fn test_output_mp_repeat() {
    let output1 = new_ucmd!().arg("/").arg("/").succeeds().stdout_move_str();
    let output1: Vec<String> = output1
        .lines()
        .map(|l| String::from(l.split_once(' ').unwrap().0))
        .collect();
    assert_eq!(3, output1.len());
    assert_eq!(output1[1], output1[2]);
}
#[test]
fn test_output_conflict_options() {
    for option in ["-i", "-T", "-P"] {
        new_ucmd!().arg("--output=source").arg(option).fails();
    }
}

#[test]
fn test_output_option() {
    new_ucmd!().arg("--output").succeeds();
    new_ucmd!().arg("--output=source,target").succeeds();
    new_ucmd!().arg("--output=invalid_option").fails();
}

#[test]
fn test_type_option() {
    new_ucmd!().args(&["-t", "ext4", "-t", "ext3"]).succeeds();
}

// ToDO: more tests...
