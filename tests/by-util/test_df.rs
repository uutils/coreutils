use crate::common::util::*;

#[test]
fn test_df_compatible_no_size_arg() {
    let (_, mut ucmd) = at_and_ucmd!();
    ucmd.arg("-a").succeeds();
 }

#[test]
fn test_df_compatible() {
    let (_, mut ucmd) = at_and_ucmd!();
    ucmd.arg("-ah").succeeds();
}

#[test]
fn test_df_compatible_type() {
    let (_, mut ucmd) = at_and_ucmd!();
    ucmd.arg("-aT").succeeds();
}

#[test]
fn test_df_compatible_si() {
    let (_, mut ucmd) = at_and_ucmd!();
    ucmd.arg("-aH").succeeds();
}

// ToDO: more tests...
