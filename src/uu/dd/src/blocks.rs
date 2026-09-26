// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore datastructures rstat rposition cflags ctable

use crate::conversion_tables::ConversionTable;
use crate::datastructures::ConversionMode;
use crate::progress::ReadStat;
use std::io;

const NEWLINE: u8 = b'\n';
const SPACE: u8 = b' ';

/// `cbs` may be huge, so `conv=block` writes the padding in pieces of this size.
const PAD_CHUNK: usize = 64 * 1024;

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

fn apply_conversion(buf: &[u8], ct: &ConversionTable) -> Vec<u8> {
    buf.iter().map(|&b| ct[b as usize]).collect()
}

/// Applies the conversion, blocking, and/or unblocking of a [`ConversionMode`].
///
/// The current `conv=block` record is kept across calls, since a record
/// may span several reads.
pub(crate) struct Converter {
    mode: ConversionMode,
    /// The length of the current record, at most `cbs`.
    record_len: usize,
    /// Whether the current record was counted as truncated.
    truncated: bool,
    /// The blocked output not yet written.
    out: Vec<u8>,
}

impl Converter {
    pub(crate) fn new(mode: ConversionMode) -> Self {
        Self {
            mode,
            record_len: 0,
            truncated: false,
            out: Vec::new(),
        }
    }

    /// Transform `buf` and pass the result to `write`, possibly in several pieces.
    ///
    /// Blocking updates the number of records truncated in `rstat`.
    pub(crate) fn convert(
        &mut self,
        buf: &[u8],
        rstat: &mut ReadStat,
        write: &mut impl FnMut(&[u8]) -> io::Result<()>,
    ) -> io::Result<()> {
        match self.mode {
            ConversionMode::ConvertOnly(ct) => write(&apply_conversion(buf, ct)),
            ConversionMode::BlockOnly(cbs) => self.block(buf, cbs, None, rstat, write),
            ConversionMode::BlockThenConvert(ct, cbs) => {
                self.block(buf, cbs, Some(ct), rstat, write)
            }
            ConversionMode::ConvertThenBlock(ct, cbs) => {
                self.block(&apply_conversion(buf, ct), cbs, None, rstat, write)
            }
            ConversionMode::UnblockOnly(cbs) => write(&unblock(buf, cbs)),
            ConversionMode::UnblockThenConvert(ct, cbs) => {
                write(&apply_conversion(&unblock(buf, cbs), ct))
            }
            ConversionMode::ConvertThenUnblock(ct, cbs) => {
                write(&unblock(&apply_conversion(buf, ct), cbs))
            }
        }
    }

    /// Pad the last record if the input did not end with a newline.
    pub(crate) fn finish(
        &mut self,
        write: &mut impl FnMut(&[u8]) -> io::Result<()>,
    ) -> io::Result<()> {
        let (cbs, ct) = match self.mode {
            ConversionMode::BlockOnly(cbs) | ConversionMode::ConvertThenBlock(_, cbs) => {
                (cbs, None)
            }
            ConversionMode::BlockThenConvert(ct, cbs) => (cbs, Some(ct)),
            _ => return Ok(()),
        };
        if self.record_len > 0 {
            self.end_record(cbs, ct, write)?;
        }
        self.flush(write)
    }

    /// Split `buf` on newlines, padding or truncating each record to `cbs` bytes.
    ///
    /// `ct` is applied to the output, padding included.
    fn block(
        &mut self,
        mut buf: &[u8],
        cbs: usize,
        ct: Option<&ConversionTable>,
        rstat: &mut ReadStat,
        write: &mut impl FnMut(&[u8]) -> io::Result<()>,
    ) -> io::Result<()> {
        loop {
            let newline = buf.iter().position(|&e| e == NEWLINE);
            let record = &buf[..newline.unwrap_or(buf.len())];
            let n = record.len().min(cbs - self.record_len);
            match ct {
                Some(ct) => self.out.extend(record[..n].iter().map(|&b| ct[b as usize])),
                None => self.out.extend_from_slice(&record[..n]),
            }
            self.record_len += n;
            if n < record.len() && !self.truncated {
                rstat.records_truncated += 1;
                self.truncated = true;
            }
            let Some(i) = newline else { break };
            self.end_record(cbs, ct, write)?;
            buf = &buf[i + 1..];
        }
        self.flush(write)
    }

    /// Pad the current record to `cbs` bytes and start a new one.
    fn end_record(
        &mut self,
        cbs: usize,
        ct: Option<&ConversionTable>,
        write: &mut impl FnMut(&[u8]) -> io::Result<()>,
    ) -> io::Result<()> {
        let pad = ct.map_or(SPACE, |ct| ct[SPACE as usize]);
        let mut remaining = cbs - self.record_len;
        while remaining > 0 {
            let n = remaining.min(PAD_CHUNK);
            self.out.resize(self.out.len() + n, pad);
            remaining -= n;
            if self.out.len() >= PAD_CHUNK {
                self.flush(write)?;
            }
        }
        self.record_len = 0;
        self.truncated = false;
        Ok(())
    }

    /// Write the blocked output.
    fn flush(&mut self, write: &mut impl FnMut(&[u8]) -> io::Result<()>) -> io::Result<()> {
        if !self.out.is_empty() {
            write(&self.out)?;
            self.out.clear();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::blocks::{Converter, PAD_CHUNK, unblock};
    use crate::datastructures::ConversionMode;
    use crate::progress::ReadStat;
    use std::io;

    const NEWLINE: u8 = b'\n';
    const SPACE: u8 = b' ';

    /// Block each slice of `reads` in turn and split the output into `cbs`-byte records.
    fn block_reads(reads: &[&[u8]], cbs: usize, rstat: &mut ReadStat) -> Vec<Vec<u8>> {
        let mode = ConversionMode::BlockOnly(cbs);
        let mut converter = Converter::new(mode);
        let mut out = Vec::new();
        let mut write = |data: &[u8]| -> io::Result<()> {
            out.extend_from_slice(data);
            Ok(())
        };
        for buf in reads {
            converter.convert(buf, rstat, &mut write).unwrap();
        }
        converter.finish(&mut write).unwrap();
        out.chunks(cbs).map(<[u8]>::to_vec).collect()
    }

    fn block(buf: &[u8], cbs: usize, rstat: &mut ReadStat) -> Vec<Vec<u8>> {
        block_reads(&[buf], cbs, rstat)
    }

    #[test]
    fn block_test_record_across_reads() {
        let mut rs = ReadStat::default();
        let res = block_reads(&[b"ab", b"cd\nef", b"ghi\n"], 4, &mut rs);

        assert_eq!(res, vec![b"abcd".to_vec(), b"efgh".to_vec()]);
        assert_eq!(rs.records_truncated, 1);
    }

    #[test]
    fn block_test_huge_cbs_pads_in_pieces() {
        let mode = ConversionMode::BlockOnly(usize::MAX);
        let mut converter = Converter::new(mode);
        let mut written = 0;
        let res = converter.convert(b"x\n", &mut ReadStat::default(), &mut |data| {
            assert!(data.len() <= 2 * PAD_CHUNK);
            written += data.len();
            if written > 4 * PAD_CHUNK {
                return Err(io::ErrorKind::WriteZero.into());
            }
            Ok(())
        });

        assert!(res.is_err());
    }

    #[test]
    fn block_test_no_nl() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8];
        let res = block(&buf, 4, &mut rs);

        assert_eq!(res, vec![vec![0u8, 1u8, 2u8, 3u8]]);
    }

    #[test]
    fn block_test_no_nl_short_record() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8];
        let res = block(&buf, 8, &mut rs);

        assert_eq!(
            res,
            vec![vec![0u8, 1u8, 2u8, 3u8, SPACE, SPACE, SPACE, SPACE]]
        );
    }

    #[test]
    fn block_test_no_nl_trunc() {
        let mut rs = ReadStat::default();
        let buf = [0u8, 1u8, 2u8, 3u8, 4u8];
        let res = block(&buf, 4, &mut rs);

        // Commented section(s) should be truncated and appear for reference only.
        assert_eq!(res, vec![vec![0u8, 1u8, 2u8, 3u8 /*, 4u8*/]]);
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

        assert_eq!(res, vec![vec![0u8, 1u8, 2u8, 3u8]]);
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
            vec![vec![0u8, 1u8, 2u8, SPACE], vec![SPACE, SPACE, SPACE, SPACE]]
        );
    }

    #[test]
    fn block_test_start_nl() {
        let mut rs = ReadStat::default();
        let buf = [NEWLINE, 0u8, 1u8, 2u8, 3u8];
        let res = block(&buf, 4, &mut rs);

        assert_eq!(
            res,
            vec![vec![SPACE, SPACE, SPACE, SPACE], vec![0u8, 1u8, 2u8, 3u8]]
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

        assert_eq!(res, vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, NEWLINE]);
    }

    #[test]
    fn unblock_test_all_space() {
        let buf = [SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE, SPACE];
        let res = unblock(&buf, 8);

        assert_eq!(res, vec![NEWLINE]);
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

        assert_eq!(res, vec![0u8, 1u8, 2u8, 3u8, NEWLINE]);
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
