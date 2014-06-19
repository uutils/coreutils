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

	let mut nodes = build_graph(...);
	let sorted = tsort(&nodes);

	print_nodes(sorted, /* stdout */);

	if nodes.length() > 0 {
		// Print error message to stderr
		print_nodes(nodes, /* stderr */);
		return 1;  // error because there were extras
	}

	return 0
}

// Build the graph from the passed file. Nodes are defined as strings in the file
// separated by whitespace (space, tab, or newline). Nodes are read as pairs. For
// each pair, there will be an edge pointing from the first node to the second node.
// If there are an odd number of nodes, the last node means nothing (but it will
// be in the graph, so if it only appears at the end of the file it will still
// appear in the output).
fn build_graph(...) -> Box<Vec<node>> {

}

// Topological sort the passed nodes. This is done by removing nodes, one by one,
// from the vector and placed into the return vector. Only nodes with in-degree
// 0 will be removed. If no nodes have in-degree 0, none will be removed.
// Thus, if this function returns and the passed vector still has nodes in it
// (i.e. if the returned vector is shorter than the initial vector was), there
// is a cycle.
fn tsort(nodes: &mut Box<Vec<node>>) -> Box<Vec<node>> {

}

// Prints all the nodes in the passed vector to the passed stream.
fn print_nodes(nodes: Box<Vec<Node>>, /* somehow a stream */) {

}

// How we represent nodes
struct node {

}