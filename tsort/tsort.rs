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

	let mut Vec<node> nodes = build_graph(...);
	tsort(&nodes);

	// Now check for cycles and print out necessary stuff

	return 0
}

fn build_graph(...) -> Vec<node> {

}

fn tsort(nodes: &mut Vec<node>) {
	
}

// How we represent nodes
struct node {

}