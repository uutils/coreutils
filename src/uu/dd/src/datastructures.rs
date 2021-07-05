// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ctable noerror

use crate::conversion_tables::*;

use std::error::Error;
use std::time;

pub struct ProgUpdate {
    pub reads_complete: u64,
    pub reads_partial: u64,
    pub writes_complete: u64,
    pub writes_partial: u64,
    pub bytes_total: u128,
    pub records_truncated: u32,
    pub duration: time::Duration,
}

pub struct ReadStat {
    pub reads_complete: u64,
    pub reads_partial: u64,
    pub records_truncated: u32,
}
impl std::ops::AddAssign for ReadStat {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            reads_complete: self.reads_complete + other.reads_complete,
            reads_partial: self.reads_partial + other.reads_partial,
            records_truncated: self.records_truncated + other.records_truncated,
        }
    }
}

pub struct WriteStat {
    pub writes_complete: u64,
    pub writes_partial: u64,
    pub bytes_total: u128,
}
impl std::ops::AddAssign for WriteStat {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            writes_complete: self.writes_complete + other.writes_complete,
            writes_partial: self.writes_partial + other.writes_partial,
            bytes_total: self.bytes_total + other.bytes_total,
        }
    }
}

type Cbs = usize;

/// Stores all Conv Flags that apply to the input
pub struct IConvFlags {
    pub ctable: Option<&'static ConversionTable>,
    pub block: Option<Cbs>,
    pub unblock: Option<Cbs>,
    pub swab: bool,
    pub sync: Option<u8>,
    pub noerror: bool,
}

/// Stores all Conv Flags that apply to the output
#[derive(Debug, PartialEq)]
pub struct OConvFlags {
    pub sparse: bool,
    pub excl: bool,
    pub nocreat: bool,
    pub notrunc: bool,
    pub fdatasync: bool,
    pub fsync: bool,
}

/// Stores all Flags that apply to the input
pub struct IFlags {
    pub cio: bool,
    pub direct: bool,
    pub directory: bool,
    pub dsync: bool,
    pub sync: bool,
    pub nocache: bool,
    pub nonblock: bool,
    pub noatime: bool,
    pub noctty: bool,
    pub nofollow: bool,
    pub nolinks: bool,
    pub binary: bool,
    pub text: bool,
    pub fullblock: bool,
    pub count_bytes: bool,
    pub skip_bytes: bool,
}

/// Stores all Flags that apply to the output
pub struct OFlags {
    pub append: bool,
    pub cio: bool,
    pub direct: bool,
    pub directory: bool,
    pub dsync: bool,
    pub sync: bool,
    pub nocache: bool,
    pub nonblock: bool,
    pub noatime: bool,
    pub noctty: bool,
    pub nofollow: bool,
    pub nolinks: bool,
    pub binary: bool,
    pub text: bool,
    pub seek_bytes: bool,
}

/// The value of the status cl-option.
/// Controls printing of transfer stats
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StatusLevel {
    Progress,
    Noxfer,
    None,
}

/// The value of count=N
/// Defaults to Reads(N)
/// if iflag=count_bytes
/// then becomes Bytes(N)
pub enum CountType {
    Reads(usize),
    Bytes(usize),
}

#[derive(Debug)]
pub enum InternalError {
    WrongInputType,
    WrongOutputType,
    InvalidConvBlockUnblockCase,
}

impl std::fmt::Display for InternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongInputType | Self::WrongOutputType => {
                write!(f, "Internal dd error: Wrong Input/Output data type")
            }
            Self::InvalidConvBlockUnblockCase => write!(
                f,
                "Internal dd error: Invalid Conversion, Block, or Unblock data"
            ),
        }
    }
}

impl Error for InternalError {}

pub mod options {
    pub const INFILE: &'static str = "if";
    pub const OUTFILE: &'static str = "of";
    pub const IBS: &'static str = "ibs";
    pub const OBS: &'static str = "obs";
    pub const BS: &'static str = "bs";
    pub const CBS: &'static str = "cbs";
    pub const COUNT: &'static str = "count";
    pub const SKIP: &'static str = "skip";
    pub const SEEK: &'static str = "seek";
    pub const STATUS: &'static str = "status";
    pub const CONV: &'static str = "conv";
    pub const IFLAG: &'static str = "iflag";
    pub const OFLAG: &'static str = "oflag";
}
