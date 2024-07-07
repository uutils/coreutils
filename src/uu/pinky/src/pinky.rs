// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) BUFSIZE gecos fullname, mesg iobuf

#[uucore::main]
use crate::platform::uumain;

pub trait Capitalize {
    fn capitalize(&self) -> String;
}

impl Capitalize for str {
    fn capitalize(&self) -> String {
        self.char_indices()
            .fold(String::with_capacity(self.len()), |mut acc, x| {
                if x.0 == 0 {
                    acc.push(x.1.to_ascii_uppercase());
                } else {
                    acc.push(x.1);
                }
                acc
            })
    }
}
