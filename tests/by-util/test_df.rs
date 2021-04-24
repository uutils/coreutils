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

// ToDO: more tests...
