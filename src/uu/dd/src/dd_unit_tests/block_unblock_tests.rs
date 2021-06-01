use super::*;

static NL: u8 = '\n' as u8;
static SPACE: u8 = ' ' as u8;

#[test]
fn block_test_no_nl()
{
    let buf = vec![0u8, 1u8, 2u8, 3u8];
    let res = block(buf, 4);

    assert_eq!(res, vec![
        vec![0u8, 1u8, 2u8, 3u8],
    ]);
}

#[test]
fn block_test_no_nl_short_rec()
{
    let buf = vec![0u8, 1u8, 2u8, 3u8];
    let res = block(buf, 8);

    assert_eq!(res, vec![
        vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
    ]);
}

#[test]
fn block_test_no_nl_trunc()
{
    let buf = vec![0u8, 1u8, 2u8, 3u8, 4u8];
    let res = block(buf, 4);

    assert_eq!(res, vec![
        vec![0u8, 1u8, 2u8, 3u8/*, 4u8*/],
    ]);
}

#[test]
fn block_test_nl_gt_cbs_trunc()
{
    let buf = vec![0u8, 1u8, 2u8, 3u8, 4u8, NL, 0u8, 1u8, 2u8, 3u8, 4u8, NL, 5u8, 6u8, 7u8, 8u8];
    let res = block(buf, 4);

    assert_eq!(res, vec![
        vec![0u8, 1u8, 2u8, 3u8],
        // gnu-dd truncates this record
        // vec![4u8, SPACE, SPACE, SPACE],
        vec![0u8, 1u8, 2u8, 3u8],
        vec![5u8, 6u8, 7u8, 8u8],
    ]);
}

#[test]
fn block_test_surrounded_nl()
{
    let buf = vec![0u8, 1u8, 2u8, 3u8, NL, 4u8, 5u8, 6u8, 7u8, 8u8];
    let res = block(buf, 8);

    assert_eq!(res, vec![
        vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
        vec![4u8, 5u8, 6u8, 7u8, 8u8, SPACE, SPACE, SPACE],
    ]);
}

#[test]
fn block_test_multiple_nl_same_block()
{
    let buf = vec![0u8, 1u8, 2u8, 3u8, NL, 4u8, NL, 5u8, 6u8, 7u8, 8u8, 9u8];
    let res = block(buf, 8);

    assert_eq!(res, vec![
        vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
        vec![4u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
        vec![5u8, 6u8, 7u8, 8u8, 9u8, SPACE, SPACE, SPACE],
    ]);
}

#[test]
fn block_test_multiple_nl_diff_block()
{
    let buf = vec![0u8, 1u8, 2u8, 3u8, NL, 4u8, 5u8, 6u8, 7u8, NL, 8u8, 9u8];
    let res = block(buf, 8);

    assert_eq!(res, vec![
        vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
        vec![4u8, 5u8, 6u8, 7u8, SPACE, SPACE, SPACE, SPACE],
        vec![8u8, 9u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
    ]);
}

#[test]
fn block_test_lone_nl_end()
{
    let buf = vec![0u8, 1u8, 2u8, 3u8, NL];
    let res = block(buf, 4);

    assert_eq!(res, vec![
        vec![0u8, 1u8, 2u8, 3u8],
    ]);
}

#[test]
fn block_test_end_nl()
{
    let buf = vec![0u8, 1u8, 2u8, NL];
    let res = block(buf, 4);

    assert_eq!(res, vec![
        vec![0u8, 1u8, 2u8, SPACE]
    ]);
}

#[test]
fn block_test_end_nl_new_block()
{
    let buf = vec![0u8, 1u8, 2u8, 3u8, NL];
    let res = block(buf, 4);

    assert_eq!(res, vec![
        vec![0u8, 1u8, 2u8, 3u8],
    ]);
}

#[test]
fn block_test_double_end_nl()
{
    let buf = vec![0u8, 1u8, 2u8, NL, NL];
    let res = block(buf, 4);

    assert_eq!(res, vec![
        vec![0u8, 1u8, 2u8, SPACE],
        vec![SPACE, SPACE, SPACE, SPACE],
    ]);
}

#[test]
fn block_test_start_nl()
{
    let buf = vec![NL, 0u8, 1u8, 2u8, 3u8];
    let res = block(buf, 4);

    assert_eq!(res, vec![
        vec![SPACE, SPACE, SPACE, SPACE],
        vec![0u8, 1u8, 2u8, 3u8],
    ]);
}

#[test]
fn block_test_double_nl()
{
    let buf = vec![0u8, 1u8, 2u8, 3u8, NL, NL, 4u8, 5u8, 6u8, 7u8];
    let res = block(buf, 8);

    assert_eq!(res, vec![
        vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
        vec![SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
        vec![4u8, 5u8, 6u8, 7u8, SPACE, SPACE, SPACE, SPACE],
    ]);
}

#[test]
fn block_test_double_nl_double_trunc()
{
    let buf = vec![0u8, 1u8, 2u8, 3u8, NL, NL, 4u8, 5u8, 6u8, 7u8, 8u8];
    let res = block(buf, 4);

    assert_eq!(res, vec![
        vec![0u8, 1u8, 2u8, 3u8],
        vec![SPACE, SPACE, SPACE, SPACE],
        vec![4u8, 5u8, 6u8, 7u8/*, 8u8*/],
    ]);
}

make_block_test!(
    block_cbs16,
    "block-cbs-16",
    File::open("./test-resources/dd-block-cbs16.test").unwrap(),
    Some(16),
    File::open("./test-resources/dd-block-cbs16.spec").unwrap()
);

make_block_test!(
    block_cbs16_as_cbs8,
    "block-cbs-16-as-cbs8",
    File::open("./test-resources/dd-block-cbs16.test").unwrap(),
    Some(8),
    File::open("./test-resources/dd-block-cbs8.spec").unwrap()
);

make_block_test!(
    block_consecutive_nl,
    "block-consecutive-nl",
    File::open("./test-resources/dd-block-consecutive-nl.test").unwrap(),
    Some(16),
    File::open("./test-resources/dd-block-consecutive-nl-cbs16.spec").unwrap()
);
