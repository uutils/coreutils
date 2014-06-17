#![crate_id(name="tsort", vers="1.0.0", author="Ben Eggers")]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Ben Eggers <ben.eggers36@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![feature(macro_rules)]

extern crate getopts;

use std::os;

static NAME: &'static str = "tsort";

#[allow(dead_code)]
fn main() { os::set_exit_status(uumain(os::args())); }

fn uumain(args: Vec<String>) -> int {
	return 0
}