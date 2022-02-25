//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore datastructures rstat rposition cflags ctable

use crate::conversion_tables::ConversionTable;
use crate::datastructures::InternalError;
use crate::progress::ReadStat;
use crate::Input;
use std::io::Read;

const NEWLINE: u8 = b'\n';
const SPACE: u8 = b' ';

/// Splits the content of buf into cbs-length blocks
/// Appends padding as specified by conv=block and cbs=N
/// Expects ascii encoded data
fn block(buf: &[u8], cbs: usize, rstat: &mut ReadStat) -> Vec<Vec<u8>> {
    let mut blocks = buf
        .split(|&e| e == NEWLINE)
        .map(|split| split.to_vec())
        .fold(Vec::new(), |mut blocks, mut split| {
            if split.len() > cbs {
                rstat.records_truncated += 1;
            }
            split.resize(cbs, SPACE);
            blocks.push(split);

            blocks
        });

    if let Some(last) = blocks.last() {
        if last.iter().all(|&e| e == SPACE) {
            blocks.pop();
        }
    }

    blocks
}

/// Trims padding from each cbs-length partition of buf
/// as specified by conv=unblock and cbs=N
/// Expects ascii encoded data
fn unblock(buf: &[u8], cbs: usize) -> Vec<u8> {
    buf.chunks(cbs).fold(Vec::new(), |mut acc, block| {
        if let Some(last_char_idx) = block.iter().rposition(|&e| e != SPACE) {
            // Include text up to last space.
            acc.extend(&block[..=last_char_idx]);
        }

        acc.push(NEWLINE);
        acc
    })
}

/// A helper for teasing out which options must be applied and in which order.
/// Some user options, such as the presence of conversion tables, will determine whether the input is assumed to be ascii. The parser sets the Input::non_ascii flag accordingly.
/// Examples:
///     - If conv=ebcdic or conv=ibm is specified then block, unblock or swab must be performed before the conversion happens since the source will start in ascii.
///     - If conv=ascii is specified then block, unblock or swab must be performed after the conversion since the source starts in ebcdic.
///     - If no conversion is specified then the source is assumed to be in ascii.
/// For more info see `info dd`
pub(crate) fn conv_block_unblock_helper<R: Read>(
    mut buf: Vec<u8>,
    i: &mut Input<R>,
    rstat: &mut ReadStat,
) -> Result<Vec<u8>, InternalError> {
    // Local Predicate Fns -------------------------------------------------
    fn should_block_then_conv<R: Read>(i: &Input<R>) -> bool {
        !i.non_ascii && i.cflags.block.is_some()
    }
    fn should_conv_then_block<R: Read>(i: &Input<R>) -> bool {
        i.non_ascii && i.cflags.block.is_some()
    }
    fn should_unblock_then_conv<R: Read>(i: &Input<R>) -> bool {
        !i.non_ascii && i.cflags.unblock.is_some()
    }
    fn should_conv_then_unblock<R: Read>(i: &Input<R>) -> bool {
        i.non_ascii && i.cflags.unblock.is_some()
    }
    fn conv_only<R: Read>(i: &Input<R>) -> bool {
        i.cflags.ctable.is_some() && i.cflags.block.is_none() && i.cflags.unblock.is_none()
    }
    // Local Helper Fns ----------------------------------------------------
    fn apply_conversion(buf: &mut [u8], ct: &ConversionTable) {
        for idx in 0..buf.len() {
            buf[idx] = ct[buf[idx] as usize];
        }
    }
    // --------------------------------------------------------------------
    if conv_only(i) {
        // no block/unblock
        let ct = i.cflags.ctable.unwrap();
        apply_conversion(&mut buf, ct);

        Ok(buf)
    } else if should_block_then_conv(i) {
        // ascii input so perform the block first
        let cbs = i.cflags.block.unwrap();

        let mut blocks = block(&buf, cbs, rstat);

        if let Some(ct) = i.cflags.ctable {
            for buf in &mut blocks {
                apply_conversion(buf, ct);
            }
        }

        let blocks = blocks.into_iter().flatten().collect();

        Ok(blocks)
    } else if should_conv_then_block(i) {
        // Non-ascii so perform the conversion first
        let cbs = i.cflags.block.unwrap();

        if let Some(ct) = i.cflags.ctable {
            apply_conversion(&mut buf, ct);
        }

        let blocks = block(&buf, cbs, rstat).into_iter().flatten().collect();

        Ok(blocks)
    } else if should_unblock_then_conv(i) {
        // ascii input so perform the unblock first
        let cbs = i.cflags.unblock.unwrap();

        let mut buf = unblock(&buf, cbs);

        if let Some(ct) = i.cflags.ctable {
            apply_conversion(&mut buf, ct);
        }

        Ok(buf)
    } else if should_conv_then_unblock(i) {
        // Non-ascii input so perform the conversion first
        let cbs = i.cflags.unblock.unwrap();

        if let Some(ct) = i.cflags.ctable {
            apply_conversion(&mut buf, ct);
        }

        let buf = unblock(&buf, cbs);

        Ok(buf)
    } else {
        // The following error should not happen, as it results from
        // insufficient command line data. This case should be caught
        // by the parser before making it this far.
        // Producing this error is an alternative to risking an unwrap call
        // on 'cbs' if the required data is not provided.
        Err(InternalError::InvalidConvBlockUnblockCase)
    }
}

#[cfg(test)]
mod tests {

    use crate::blocks::{block, unblock};
    use crate::progress::ReadStat;

    const NEWLINE: u8 = b'\n';
    const SPACE: u8 = b' ';

    #[test]
    fn block_test_no_nl() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8];
        let res = block(&buf, 4, &mut rs);

        assert_eq!(res, vec![vec![0u8, 1u8, 2u8, 3u8],]);
    }

    #[test]
    fn block_test_no_nl_short_record() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8];
        let res = block(&buf, 8, &mut rs);

        assert_eq!(
            res,
            vec![vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],]
        );
    }

    #[test]
    fn block_test_no_nl_trunc() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8, 4u8];
        let res = block(&buf, 4, &mut rs);

        // Commented section(s) should be truncated and appear for reference only.
        assert_eq!(res, vec![vec![0u8, 1u8, 2u8, 3u8 /*, 4u8*/],]);
        assert_eq!(rs.records_truncated, 1);
    }

    #[test]
    fn block_test_nl_gt_cbs_trunc() {
        let mut rs = ReadStat::default();
        let buf = [
            0u8, 1u8, 2u8, 3u8, 4u8, NEWLINE, 0u8, 1u8, 2u8, 3u8, 4u8, NEWLINE, 5u8, 6u8, 7u8, 8u8,
        ];
        let res = block(&buf, 4, &mut rs);

        assert_eq!(
            res,
            vec![
                // Commented section(s) should be truncated and appear for reference only.
                vec![0u8, 1u8, 2u8, 3u8],
                // vec![4u8, SPACE, SPACE, SPACE],
                vec![0u8, 1u8, 2u8, 3u8],
                // vec![4u8, SPACE, SPACE, SPACE],
                vec![5u8, 6u8, 7u8, 8u8],
            ]
        );
        assert_eq!(rs.records_truncated, 2);
    }

    #[test]
    fn block_test_surrounded_nl() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8, NEWLINE, 4u8, 5u8, 6u8, 7u8, 8u8];
        let res = block(&buf, 8, &mut rs);

        assert_eq!(
            res,
            vec![
                vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
                vec![4u8, 5u8, 6u8, 7u8, 8u8, SPACE, SPACE, SPACE],
            ]
        );
    }

    #[test]
    fn block_test_multiple_nl_same_cbs_block() {
        let mut rs = ReadStat::default();
        let buf = [
            0u8, 1u8, 2u8, 3u8, NEWLINE, 4u8, NEWLINE, 5u8, 6u8, 7u8, 8u8, 9u8,
        ];
        let res = block(&buf, 8, &mut rs);

        assert_eq!(
            res,
            vec![
                vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
                vec![4u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
                vec![5u8, 6u8, 7u8, 8u8, 9u8, SPACE, SPACE, SPACE],
            ]
        );
    }

    #[test]
    fn block_test_multiple_nl_diff_cbs_block() {
        let mut rs = ReadStat::default();
        let buf = [
            0u8, 1u8, 2u8, 3u8, NEWLINE, 4u8, 5u8, 6u8, 7u8, NEWLINE, 8u8, 9u8,
        ];
        let res = block(&buf, 8, &mut rs);

        assert_eq!(
            res,
            vec![
                vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
                vec![4u8, 5u8, 6u8, 7u8, SPACE, SPACE, SPACE, SPACE],
                vec![8u8, 9u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
            ]
        );
    }

    #[test]
    fn block_test_end_nl_diff_cbs_block() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8, NEWLINE];
        let res = block(&buf, 4, &mut rs);

        assert_eq!(res, vec![vec![0u8, 1u8, 2u8, 3u8],]);
    }

    #[test]
    fn block_test_end_nl_same_cbs_block() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, NEWLINE];
        let res = block(&buf, 4, &mut rs);

        assert_eq!(res, vec![vec![0u8, 1u8, 2u8, SPACE]]);
    }

    #[test]
    fn block_test_double_end_nl() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, NEWLINE, NEWLINE];
        let res = block(&buf, 4, &mut rs);

        assert_eq!(
            res,
            vec![vec![0u8, 1u8, 2u8, SPACE], vec![SPACE, SPACE, SPACE, SPACE],]
        );
    }

    #[test]
    fn block_test_start_nl() {
        let mut rs = ReadStat::default();
        let buf = [NEWLINE, 0u8, 1u8, 2u8, 3u8];
        let res = block(&buf, 4, &mut rs);

        assert_eq!(
            res,
            vec![vec![SPACE, SPACE, SPACE, SPACE], vec![0u8, 1u8, 2u8, 3u8],]
        );
    }

    #[test]
    fn block_test_double_surrounded_nl_no_trunc() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8, NEWLINE, NEWLINE, 4u8, 5u8, 6u8, 7u8];
        let res = block(&buf, 8, &mut rs);

        assert_eq!(
            res,
            vec![
                vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
                vec![SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
                vec![4u8, 5u8, 6u8, 7u8, SPACE, SPACE, SPACE, SPACE],
            ]
        );
    }

    #[test]
    fn block_test_double_surrounded_nl_double_trunc() {
        let mut rs = ReadStat::default();
        let buf = [
            0u8, 1u8, 2u8, 3u8, NEWLINE, NEWLINE, 4u8, 5u8, 6u8, 7u8, 8u8,
        ];
        let res = block(&buf, 4, &mut rs);

        assert_eq!(
            res,
            vec![
                // Commented section(s) should be truncated and appear for reference only.
                vec![0u8, 1u8, 2u8, 3u8],
                vec![SPACE, SPACE, SPACE, SPACE],
                vec![4u8, 5u8, 6u8, 7u8 /*, 8u8*/],
            ]
        );
        assert_eq!(rs.records_truncated, 1);
    }

    #[test]
    fn unblock_test_full_cbs() {
        let buf = [0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8];
        let res = unblock(&buf, 8);

        assert_eq!(res, vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, NEWLINE],);
    }

    #[test]
    fn unblock_test_all_space() {
        let buf = [SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE];
        let res = unblock(&buf, 8);

        assert_eq!(res, vec![NEWLINE],);
    }

    #[test]
    fn unblock_test_decoy_spaces() {
        let buf = [0u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, 7u8];
        let res = unblock(&buf, 8);

        assert_eq!(
            res,
            vec![0u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, 7u8, NEWLINE],
        );
    }

    #[test]
    fn unblock_test_strip_single_cbs() {
        let buf = [0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE];
        let res = unblock(&buf, 8);

        assert_eq!(res, vec![0u8, 1u8, 2u8, 3u8, NEWLINE],);
    }

    #[test]
    fn unblock_test_strip_multi_cbs() {
        let buf = vec![
            vec![0u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
            vec![0u8, 1u8, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE],
            vec![0u8, 1u8, 2u8, SPACE, SPACE, SPACE, SPACE, SPACE],
            vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        let res = unblock(&buf, 8);

        let exp = vec![
            vec![0u8, NEWLINE],
            vec![0u8, 1u8, NEWLINE],
            vec![0u8, 1u8, 2u8, NEWLINE],
            vec![0u8, 1u8, 2u8, 3u8, NEWLINE],
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        assert_eq!(res, exp);
    }
}
