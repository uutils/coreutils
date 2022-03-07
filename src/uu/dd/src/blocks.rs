//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore datastructures rstat rposition cflags ctable

use crate::conversion_tables::ConversionTable;
use crate::datastructures::ConversionMode;
use crate::progress::ReadStat;
use crate::Input;
use std::io::Read;

const NEWLINE: u8 = b'\n';
const SPACE: u8 = b' ';

/// Split a slice into chunks, padding or truncating as necessary.
///
/// The slice `buf` is split on newlines, then each block is resized
/// to `cbs` bytes, padding with spaces if necessary. This function
/// expects the input bytes to be ASCII-encoded.
///
/// If `sync` is true and there has been at least one partial record
/// read from the input (as indicated in `rstat`), then leave an
/// all-spaces block at the end. Otherwise, remove the last block if
/// it is all spaces.
fn block(buf: &[u8], cbs: usize, sync: bool, rstat: &mut ReadStat) -> Vec<Vec<u8>> {
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

    // If `sync` is true and there has been at least one partial
    // record read from the input, then leave the all-spaces block at
    // the end. Otherwise, remove it.
    if let Some(last) = blocks.last() {
        if (!sync || rstat.reads_partial == 0) && last.iter().all(|&e| e == SPACE) {
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

/// Given the various command-line parameters, determine the conversion mode.
///
/// The `conv` command-line option can take many different values,
/// each of which may combine with others. For example, `conv=ascii`,
/// `conv=lcase`, `conv=sync`, and so on. The arguments to this
/// function represent the settings of those various command-line
/// parameters. This function translates those settings to a
/// [`ConversionMode`].
fn conversion_mode(
    ctable: Option<&ConversionTable>,
    block: Option<usize>,
    unblock: Option<usize>,
    non_ascii: bool,
    is_sync: bool,
) -> Option<ConversionMode> {
    match (ctable, block, unblock) {
        (Some(ct), None, None) => Some(ConversionMode::ConvertOnly(ct)),
        (Some(ct), Some(cbs), None) => {
            if non_ascii {
                Some(ConversionMode::ConvertThenBlock(ct, cbs, is_sync))
            } else {
                Some(ConversionMode::BlockThenConvert(ct, cbs, is_sync))
            }
        }
        (Some(ct), None, Some(cbs)) => {
            if non_ascii {
                Some(ConversionMode::ConvertThenUnblock(ct, cbs))
            } else {
                Some(ConversionMode::UnblockThenConvert(ct, cbs))
            }
        }
        (None, Some(cbs), None) => Some(ConversionMode::BlockOnly(cbs, is_sync)),
        (None, None, Some(cbs)) => Some(ConversionMode::UnblockOnly(cbs)),
        (None, None, None) => None,
        // The remaining variants should never happen because the
        // argument parsing above should result in an error before
        // getting to this line of code.
        _ => unreachable!(),
    }
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
) -> Vec<u8> {
    // TODO This function has a mutable input `buf` but also returns a
    // completely new `Vec`; that seems fishy. Could we either make
    // the input immutable or make the function not return anything?

    fn apply_conversion(buf: &mut [u8], ct: &ConversionTable) {
        for idx in 0..buf.len() {
            buf[idx] = ct[buf[idx] as usize];
        }
    }

    let mode = conversion_mode(
        i.cflags.ctable,
        i.cflags.block,
        i.cflags.unblock,
        i.non_ascii,
        i.cflags.sync.is_some(),
    )
    .unwrap();
    match &mode {
        ConversionMode::ConvertOnly(ct) => {
            apply_conversion(&mut buf, ct);
            buf
        }
        ConversionMode::BlockThenConvert(ct, cbs, sync) => {
            let mut blocks = block(&buf, *cbs, *sync, rstat);
            for buf in &mut blocks {
                apply_conversion(buf, ct);
            }
            blocks.into_iter().flatten().collect()
        }
        ConversionMode::ConvertThenBlock(ct, cbs, sync) => {
            apply_conversion(&mut buf, ct);
            block(&buf, *cbs, *sync, rstat)
                .into_iter()
                .flatten()
                .collect()
        }
        ConversionMode::BlockOnly(cbs, sync) => block(&buf, *cbs, *sync, rstat)
            .into_iter()
            .flatten()
            .collect(),
        ConversionMode::UnblockThenConvert(ct, cbs) => {
            let mut buf = unblock(&buf, *cbs);
            apply_conversion(&mut buf, ct);
            buf
        }
        ConversionMode::ConvertThenUnblock(ct, cbs) => {
            apply_conversion(&mut buf, ct);
            unblock(&buf, *cbs)
        }
        ConversionMode::UnblockOnly(cbs) => unblock(&buf, *cbs),
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
        let res = block(&buf, 4, false, &mut rs);

        assert_eq!(res, vec![vec![0u8, 1u8, 2u8, 3u8],]);
    }

    #[test]
    fn block_test_no_nl_short_record() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8];
        let res = block(&buf, 8, false, &mut rs);

        assert_eq!(
            res,
            vec![vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE],]
        );
    }

    #[test]
    fn block_test_no_nl_trunc() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8, 4u8];
        let res = block(&buf, 4, false, &mut rs);

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
        let res = block(&buf, 4, false, &mut rs);

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
        let res = block(&buf, 8, false, &mut rs);

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
        let res = block(&buf, 8, false, &mut rs);

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
        let res = block(&buf, 8, false, &mut rs);

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
        let res = block(&buf, 4, false, &mut rs);

        assert_eq!(res, vec![vec![0u8, 1u8, 2u8, 3u8],]);
    }

    #[test]
    fn block_test_end_nl_same_cbs_block() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, NEWLINE];
        let res = block(&buf, 4, false, &mut rs);

        assert_eq!(res, vec![vec![0u8, 1u8, 2u8, SPACE]]);
    }

    #[test]
    fn block_test_double_end_nl() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, NEWLINE, NEWLINE];
        let res = block(&buf, 4, false, &mut rs);

        assert_eq!(
            res,
            vec![vec![0u8, 1u8, 2u8, SPACE], vec![SPACE, SPACE, SPACE, SPACE],]
        );
    }

    #[test]
    fn block_test_start_nl() {
        let mut rs = ReadStat::default();
        let buf = [NEWLINE, 0u8, 1u8, 2u8, 3u8];
        let res = block(&buf, 4, false, &mut rs);

        assert_eq!(
            res,
            vec![vec![SPACE, SPACE, SPACE, SPACE], vec![0u8, 1u8, 2u8, 3u8],]
        );
    }

    #[test]
    fn block_test_double_surrounded_nl_no_trunc() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8, NEWLINE, NEWLINE, 4u8, 5u8, 6u8, 7u8];
        let res = block(&buf, 8, false, &mut rs);

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
        let res = block(&buf, 4, false, &mut rs);

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
