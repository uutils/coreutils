// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

pub static BEL: u8 = 0x7;
pub static BS: u8 = 0x8;
pub static HT: u8 = 0x9;
pub static LF: u8 = 0xA;
pub static VT: u8 = 0xB;
pub static FF: u8 = 0xC;
pub static CR: u8 = 0xD;
pub static SPACE: u8 = 0x20;
pub static SPACES: &[u8] = &[HT, LF, VT, FF, CR, SPACE];
pub static BLANK: &[u8] = &[HT, SPACE];
