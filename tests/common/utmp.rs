// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::mem::size_of;
use std::path::Path;

use uucore::utmpx;

const GLIBC_RESERVED_SIZE: usize = 20;
const UT_ADDR_V6_WORDS: usize = 4;
const UT_TYPE_PADDING: usize = size_of::<i32>() - size_of::<i16>();

pub(crate) struct LinuxGlibcUtmpRecord {
    record_type: i16,
    pid: i32,
    line: [u8; utmpx::UT_LINESIZE],
    id: [u8; utmpx::UT_IDSIZE],
    user: [u8; utmpx::UT_NAMESIZE],
    host: [u8; utmpx::UT_HOSTSIZE],
    termination: i16,
    exit: i16,
    timestamp: i32,
}

impl LinuxGlibcUtmpRecord {
    pub(crate) fn new(
        record_type: i16,
        pid: i32,
        line: &str,
        id: &str,
        user: &str,
        host: &str,
    ) -> Self {
        Self {
            record_type,
            pid,
            line: fixed_field(line),
            id: fixed_field(id),
            user: fixed_field(user),
            host: fixed_field(host),
            termination: 0,
            exit: 0,
            timestamp: 1_716_371_201,
        }
    }

    #[cfg(feature = "who")]
    pub(crate) fn with_exit_status(mut self, termination: i16, exit: i16) -> Self {
        self.termination = termination;
        self.exit = exit;
        self
    }

    fn encode_into(&self, bytes: &mut Vec<u8>) {
        let start = bytes.len();
        bytes.extend_from_slice(&self.record_type.to_ne_bytes());
        // glibc aligns ut_pid after the 16-bit ut_type field.
        bytes.resize(bytes.len() + UT_TYPE_PADDING, 0);
        bytes.extend_from_slice(&self.pid.to_ne_bytes());
        bytes.extend_from_slice(&self.line);
        bytes.extend_from_slice(&self.id);
        bytes.extend_from_slice(&self.user);
        bytes.extend_from_slice(&self.host);
        bytes.extend_from_slice(&self.termination.to_ne_bytes());
        bytes.extend_from_slice(&self.exit.to_ne_bytes());
        bytes.extend_from_slice(&0_i32.to_ne_bytes());
        bytes.extend_from_slice(&self.timestamp.to_ne_bytes());
        bytes.extend_from_slice(&0_i32.to_ne_bytes());
        for _ in 0..UT_ADDR_V6_WORDS {
            bytes.extend_from_slice(&0_i32.to_ne_bytes());
        }
        bytes.resize(bytes.len() + GLIBC_RESERVED_SIZE, 0);
        assert_eq!(bytes.len() - start, size_of::<libc::utmpx>());
    }
}

fn fixed_field<const N: usize>(value: &str) -> [u8; N] {
    assert!(value.len() <= N);
    let mut field = [0; N];
    field[..value.len()].copy_from_slice(value.as_bytes());
    field
}

pub(crate) fn write_linux_glibc_utmp(path: &Path, records: &[LinuxGlibcUtmpRecord]) {
    let mut bytes = Vec::with_capacity(size_of::<libc::utmpx>() * records.len());
    for record in records {
        record.encode_into(&mut bytes);
    }
    std::fs::write(path, bytes).unwrap();
}
