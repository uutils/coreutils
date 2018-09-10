#![crate_name = "uu_tsort"]

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

#[macro_use]
extern crate uucore;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Read};
use std::path::Path;

static NAME: &str = "tsort";
static VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f),
    };

    if matches.opt_present("h") {
        println!("{} {}", NAME, VERSION);
        println!();
        println!("Usage:");
        println!("  {} [OPTIONS] FILE", NAME);
        println!();
        println!("{}", opts.usage("Topological sort the strings in FILE. Strings are defined as any sequence of tokens separated by whitespace (tab, space, or newline). If FILE is not passed in, stdin is used instead."));
        return 0;
    }

    if matches.opt_present("V") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    let files = matches.free.clone();
    let input = if files.len() > 1 {
        crash!(1, "{}, extra operand '{}'", NAME, matches.free[1]);
    } else if files.is_empty() {
        "-".to_owned()
    } else {
        files[0].clone()
    };

    let mut stdin_buf;
    let mut file_buf;
    let mut reader = BufReader::new(if input == "-" {
        stdin_buf = stdin();
        &mut stdin_buf as &mut Read
    } else {
        file_buf = match File::open(Path::new(&input)) {
            Ok(a) => a,
            _ => {
                show_error!("{}: No such file or directory", input);
                return 1;
            }
        };
        &mut file_buf as &mut Read
    });

    let mut g = Graph::new();
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(_) => {
                let tokens: Vec<String> = line.trim_right()
                    .split_whitespace()
                    .map(|s| s.to_owned())
                    .collect();
                if tokens.is_empty() {
                    break;
                }
                for ab in tokens.chunks(2) {
                    match ab.len() {
                        2 => g.add_edge(&ab[0], &ab[1]),
                        _ => crash!(1, "{}: input contains an odd number of tokens", input),
                    }
                }
            }
            _ => break,
        }
    }

    g.run_tsort();

    if !g.is_acyclic() {
        crash!(1, "{}, input contains a loop:", input);
    }

    for x in &g.result {
        println!("{}", x);
    }

    0
}

// We use String as a representation of node here
// but using integer may improve performance.
struct Graph {
    in_edges: HashMap<String, HashSet<String>>,
    out_edges: HashMap<String, Vec<String>>,
    result: Vec<String>,
}

impl Graph {
    fn new() -> Graph {
        Graph {
            in_edges: HashMap::new(),
            out_edges: HashMap::new(),
            result: vec![],
        }
    }

    fn has_node(&self, n: &str) -> bool {
        self.in_edges.contains_key(n)
    }

    fn has_edge(&self, from: &str, to: &str) -> bool {
        self.in_edges[to].contains(from)
    }

    fn init_node(&mut self, n: &str) {
        self.in_edges.insert(n.to_string(), HashSet::new());
        self.out_edges.insert(n.to_string(), vec![]);
    }

    fn add_edge(&mut self, from: &str, to: &str) {
        if !self.has_node(to) {
            self.init_node(to);
        }

        if !self.has_node(from) {
            self.init_node(from);
        }

        if !self.has_edge(from, to) {
            self.in_edges.get_mut(to).unwrap().insert(from.to_string());
            self.out_edges.get_mut(from).unwrap().push(to.to_string());
        }
    }

    // Kahn's algorithm
    // O(|V|+|E|)
    fn run_tsort(&mut self) {
        let mut start_nodes = vec![];
        for (n, edges) in &self.in_edges {
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
        for edges in self.out_edges.values() {
            if !edges.is_empty() {
                return false;
            }
        }
        true
    }
}
