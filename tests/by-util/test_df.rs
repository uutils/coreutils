use crate::common::util::*;

#[test]
fn test_df_compatible_no_size_arg() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg("-a").succeeds();
 }

#[test]
fn test_df_compatible() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg("-ah").succeeds();
}

#[test]
fn test_df_compatible_type() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg("-aT").succeeds();
}

#[test]
fn test_df_compatible_si() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg("-aH").succeeds();
}

// ToDO: more tests...
