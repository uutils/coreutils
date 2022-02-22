// spell-checker:ignore itotal iused iavail ipcent pcent
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

#[test]
fn test_df_specified_output_selectors() {
    new_ucmd!().arg("--output=target,pcent,avail,used,size,ipcent,iavail,iused,itotal,fstype,source").arg("-total").succeeds().
    stdout_only("Mounted on        Use%    Available         Used    1k-blocks IUse%        IFree        IUsed       Inodes Type  Filesystem       \n");
}

/// Test that the order of rows in the table does not change across executions.
#[test]
fn test_order_same() {
    let output1 = new_ucmd!()
        .arg("--output=source")
        .succeeds()
        .stdout_move_str();
    let output2 = new_ucmd!()
        .arg("--output=source")
        .succeeds()
        .stdout_move_str();
    assert_eq!(output1, output2);
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
    new_ucmd!().arg("--output=size,size").fails();
}

#[test]
fn test_type_option() {
    new_ucmd!().args(&["-t", "ext4", "-t", "ext3"]).succeeds();
}

// ToDO: more tests...
