//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

pub static BEL: char = '\u{0007}';
pub static BS: char = '\u{0008}';
pub static HT: char = '\u{0009}';
pub static LF: char = '\u{000A}';
pub static VT: char = '\u{000B}';
pub static FF: char = '\u{000C}';
pub static CR: char = '\u{000D}';
pub static SPACE: char = '\u{0020}';
pub static SPACES: &[char] = &[HT, LF, VT, FF, CR, SPACE];
pub static BLANK: &[char] = &[SPACE, HT];
