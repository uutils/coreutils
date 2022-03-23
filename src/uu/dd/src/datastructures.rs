// This file is part of the uutils coreutils package.
//
// (c) Tyler Steele <tyler.steele@protonmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore ctable, outfile, iseek, oseek

use std::error::Error;

use uucore::error::UError;

use crate::conversion_tables::*;

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

/// The value of count=N
/// Defaults to Reads(N)
/// if iflag=count_bytes
/// then becomes Bytes(N)
#[derive(Debug, PartialEq)]
pub enum CountType {
    Reads(u64),
    Bytes(u64),
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
    pub const ISEEK: &str = "iseek";
    pub const OSEEK: &str = "oseek";
    pub const STATUS: &str = "status";
    pub const CONV: &str = "conv";
    pub const IFLAG: &str = "iflag";
    pub const OFLAG: &str = "oflag";
}
