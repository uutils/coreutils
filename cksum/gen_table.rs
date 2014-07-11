/*
* This file is part of the uutils coreutils package.
*
* (c) Arcterus <arcterus@mail.com>
* (c) Michael Gehring <mg@ebfe.org>
*
* For the full copyright and license information, please view the LICENSE
* file that was distributed with this source code.
*/

use std::io;

static CRC_TABLE_LEN: uint = 256;

fn main() {
    let mut table = Vec::with_capacity(CRC_TABLE_LEN);
    for num in range(0, CRC_TABLE_LEN) {
        table.push(crc_entry(num as u8) as u32);
    }
    let mut file = io::File::open_mode(&Path::new("crc_table.rs"), io::Truncate, io::Write).unwrap();
    let output = format!("/* auto-generated (DO NOT EDIT) */

pub static CRC_TABLE: [u32, ..{}] = {};", CRC_TABLE_LEN, table);
    file.write_line(output.as_slice()).unwrap();
}

#[inline]
fn crc_entry(input: u8) -> u32 {
    let mut crc = input as u32 << 24;

    for _ in range(0u, 8) {
        if crc & 0x80000000 != 0 {
            crc <<= 1;
            crc ^= 0x04c11db7;
        } else {
            crc <<= 1;
        }
    }

    crc
}
