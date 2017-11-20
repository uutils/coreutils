use common::util::*;

#[test]
fn test_df_compatible() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.arg("-ah").run();
    assert!(result.success);
}
// TODO
