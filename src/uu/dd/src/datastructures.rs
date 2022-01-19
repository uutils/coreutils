// This file is part of the uutils coreutils package.
//
// (c) Tyler Steele <tyler.steele@protonmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore ctable, outfile

use std::error::Error;
use std::time;

use uucore::error::UError;

use crate::conversion_tables::*;

pub struct ProgUpdate {
    pub read_stat: ReadStat,
    pub write_stat: WriteStat,
    pub duration: time::Duration,
}

#[derive(Clone, Copy, Default)]
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

#[derive(Clone, Copy)]
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
#[derive(Debug, Default, PartialEq)]
pub struct IConvFlags {
    pub ctable: Option<&'static ConversionTable>,
    pub block: Option<Cbs>,
    pub unblock: Option<Cbs>,
    pub swab: bool,
    pub sync: Option<u8>,
    pub noerror: bool,
}

/// Stores all Conv Flags that apply to the output
#[derive(Debug, Default, PartialEq)]
pub struct OConvFlags {
    pub sparse: bool,
    pub excl: bool,
    pub nocreat: bool,
    pub notrunc: bool,
    pub fdatasync: bool,
    pub fsync: bool,
}

/// Stores all Flags that apply to the input
#[derive(Debug, Default, PartialEq)]
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
#[derive(Debug, Default, PartialEq)]
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
#[derive(Debug, PartialEq)]
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
            Self::InvalidConvBlockUnblockCase => {
                write!(f, "Invalid Conversion, Block, or Unblock data")
            }
        }
    }
}

impl Error for InternalError {}
impl UError for InternalError {}

pub mod options {
    pub const INFILE: &str = "if";
    pub const OUTFILE: &str = "of";
    pub const IBS: &str = "ibs";
    pub const OBS: &str = "obs";
    pub const BS: &str = "bs";
    pub const CBS: &str = "cbs";
    pub const COUNT: &str = "count";
    pub const SKIP: &str = "skip";
    pub const SEEK: &str = "seek";
    pub const STATUS: &str = "status";
    pub const CONV: &str = "conv";
    pub const IFLAG: &str = "iflag";
    pub const OFLAG: &str = "oflag";
}
