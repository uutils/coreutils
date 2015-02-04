#![crate_name = "tsort"]
#![feature(collections, core, io, libc, path, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Ben Eggers <ben.eggers36@gmail.com>
 * (c) Akira Hayakawa <ruby.wktk@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;

use std::old_io as io;
use std::collections::{HashSet, HashMap};

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "tsort";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> isize {
    let opts = [
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f)
    };

    if matches.opt_present("h") {
        println!("{} v{}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {} [OPTIONS] FILE", NAME);
        println!("");
        io::print(getopts::usage("Topological sort the strings in FILE. Strings are defined as any sequence of tokens separated by whitespace (tab, space, or newline). If FILE is not passed in, stdin is used instead.", &opts).as_slice());
        return 0;
    }

    if matches.opt_present("V") {
        println!("{} v{}", NAME, VERSION);
        return 0;
    }

    let files = matches.free.clone();
    let input = if files.len() > 1 {
        crash!(1, "{}, extra operand '{}'", NAME, matches.free[1]);
    } else if files.is_empty() {
        "-".to_string()
    } else {
        files[0].to_string()
    };

    let mut stdin_buf;
    let mut file_buf;
    let mut reader = io::BufferedReader::new(
        if input.as_slice() == "-" {
            stdin_buf = io::stdio::stdin_raw();
            &mut stdin_buf as &mut Reader
        } else {
            file_buf = match io::File::open(&Path::new(input.as_slice())) {
                Ok(a) => a,
                _ => {
                    show_error!("{}: No such file or directory", input);
                    return 1;
                }
            };
            &mut file_buf as &mut Reader
        }
    );

    let mut g = Graph::new();
    loop {
        match reader.read_line() {
            Ok(line) => {
                let ab: Vec<&str> = line.as_slice().trim_right_matches('\n').split(' ').collect();
                if ab.len() > 2 {
                    crash!(1, "{}: input contains an odd number of tokens", input);
                }
                g.add_edge(&ab[0].to_string(), &ab[1].to_string());
            },
            _ => break
        }
    }

    g.run_tsort();

    if !g.is_acyclic() {
        crash!(1, "{}, input contains a loop:", input);
    }

    for x in g.result.iter() {
        println!("{}", x);
    }

    return 0
}

// We use String as a representation of node here
// but using integer may improve performance.
struct Graph {
    in_edges: HashMap<String, HashSet<String>>,
    out_edges: HashMap<String, Vec<String>>,
    result: Vec<String>
}

impl Graph {
    fn new() -> Graph {
        Graph {
            in_edges: HashMap::new(),
            out_edges: HashMap::new(),
            result: vec!(),
        }
    }

    fn has_node(&self, n: &String) -> bool {
        self.in_edges.contains_key(n)
    }

    fn has_edge(&self, from: &String, to: &String) -> bool {
        self.in_edges.get(to).unwrap().contains(from)
    }

    fn init_node(&mut self, n: &String) {
        self.in_edges.insert(n.clone(), HashSet::new());
        self.out_edges.insert(n.clone(), vec!());
    }

    fn add_edge(&mut self, from: &String,  to: &String) {
        if !self.has_node(to) {
            self.init_node(to);
        }

        if !self.has_node(from) {
            self.init_node(from);
        }

        if !self.has_edge(from, to) {
            self.in_edges.get_mut(to).unwrap().insert(from.clone());
            self.out_edges.get_mut(from).unwrap().push(to.clone());
        }
    }

    // Kahn's algorithm
    // O(|V|+|E|)
    fn run_tsort(&mut self) {
        let mut start_nodes = vec!();
        for (n, edges) in self.in_edges.iter() {
            if edges.is_empty() {
                start_nodes.push(n.clone());
            }
        }

        while !start_nodes.is_empty() {
            let n = start_nodes.remove(0);

            self.result.push(n.clone());

            let n_out_edges = self.out_edges.get_mut(&n).unwrap();
            for m in n_out_edges.iter() {
                let m_in_edges = self.in_edges.get_mut(m).unwrap();
                m_in_edges.remove(&n);

                // If m doesn't have other in-coming edges add it to start_nodes
                if m_in_edges.is_empty() {
                    start_nodes.push(m.clone());
                }
            }
            n_out_edges.clear();
        }
    }

    fn is_acyclic(&self) -> bool {
        for (_, edges) in self.out_edges.iter() {
            if !edges.is_empty() {
                return false
            }
        }
        true
    }
}
