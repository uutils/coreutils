use crate::common::util::*;

#[test]
fn test_df_compatible_no_size_arg() {
    new_ucmd!().arg("-a").succeeds();
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

// ToDO: more tests...
