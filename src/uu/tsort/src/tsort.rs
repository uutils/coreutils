// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//spell-checker:ignore TAOCP indegree
use clap::{Arg, Command};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::ffi::OsString;
use std::path::Path;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{UError, UResult};
use uucore::{format_usage, show};

use uucore::translate;

mod options {
    pub const FILE: &str = "file";
}

#[derive(Debug, Error)]
enum TsortError {
    /// The input file is actually a directory.
    #[error("{input}: {message}", input = .0, message = translate!("tsort-error-is-dir"))]
    IsDir(String),

    /// The number of tokens in the input data is odd.
    ///
    /// The list of edges must be even because each edge has two
    /// components: a source node and a target node.
    #[error("{input}: {message}", input = .0.maybe_quote(), message = translate!("tsort-error-odd"))]
    NumTokensOdd(String),

    /// The graph contains a cycle.
    #[error("{input}: {message}", input = .0, message = translate!("tsort-error-loop"))]
    Loop(String),

    /// A particular node in a cycle. (This is mainly used for printing.)
    #[error("{0}")]
    LoopNode(String),
}

impl UError for TsortError {}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let input = matches
        .get_one::<OsString>(options::FILE)
        .expect("Value is required by clap");

    let data = if input == "-" {
        let stdin = std::io::stdin();
        std::io::read_to_string(stdin)?
    } else {
        let path = Path::new(input);
        if path.is_dir() {
            return Err(TsortError::IsDir(input.to_string_lossy().to_string()).into());
        }
        std::fs::read_to_string(path)?
    };

    // Create the directed graph from pairs of tokens in the input data.
    let input_name = input.to_string_lossy().to_string();
    let mut g = Graph::new(input_name.clone());
    let mut tokens = data.split_whitespace();
    loop {
        let Some(a) = tokens.next() else { break };
        let Some(b) = tokens.next() else {
            return Err(TsortError::NumTokensOdd(input_name).into());
        };
        g.add_edge(a, b);
    }

    g.run_tsort();
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(format_usage(&translate!("tsort-usage")))
        .about(translate!("tsort-about"))
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE)
                .default_value("-")
                .hide(true)
                .value_parser(clap::value_parser!(OsString))
                .value_hint(clap::ValueHint::FilePath),
        )
}

/// Find the element `x` in `vec` and remove it, returning its index.
fn remove<T>(vec: &mut Vec<T>, x: T) -> Option<usize>
where
    T: PartialEq,
{
    vec.iter().position(|item| *item == x).map(|i| {
        vec.swap_remove(i);
        i
    })
}

// We use String as a representation of node here
// but using integer may improve performance.
#[derive(Default)]
struct Node<'input> {
    successor_names: Vec<&'input str>,
    predecessor_count: usize,
}

impl<'input> Node<'input> {
    fn add_successor(&mut self, successor_name: &'input str) {
        self.successor_names.push(successor_name);
    }
}

struct Graph<'input> {
    name: String,
    nodes: HashMap<&'input str, Node<'input>>,
}

impl<'input> Graph<'input> {
    fn new(name: String) -> Graph<'input> {
        Self {
            name,
            nodes: HashMap::default(),
        }
    }

    fn add_node(&mut self, name: &'input str) {
        self.nodes.entry(name).or_default();
    }

    fn add_edge(&mut self, from: &'input str, to: &'input str) {
        self.add_node(from);
        if from != to {
            self.add_node(to);

            let from_node = self.nodes.get_mut(from).unwrap();
            from_node.add_successor(to);

            let to_node = self.nodes.get_mut(to).unwrap();
            to_node.predecessor_count += 1;
        }
    }

    fn remove_edge(&mut self, u: &'input str, v: &'input str) {
        remove(&mut self.nodes.get_mut(u).unwrap().successor_names, v);
        self.nodes.get_mut(v).unwrap().predecessor_count -= 1;
    }

    /// Implementation of algorithm T from TAOCP (Don. Knuth), vol. 1.
    fn run_tsort(&mut self) {
        // First, we find nodes that have no prerequisites (independent nodes).
        // If no such node exists, then there is a cycle.
        let mut independent_nodes_queue: BinaryHeap<Reverse<&'input str>> = self
            .nodes
            .iter()
            .filter_map(|(&name, node)| {
                if node.predecessor_count == 0 {
                    Some(Reverse(name))
                } else {
                    None
                }
            })
            .collect();

        while !self.nodes.is_empty() {
            // Get the next node (breaking any cycles necessary to do so).
            let v = self.find_next_node(&mut independent_nodes_queue);
            println!("{v}");
            if let Some(node_to_process) = self.nodes.remove(v) {
                for successor_name in node_to_process.successor_names {
                    let successor_node = self.nodes.get_mut(successor_name).unwrap();
                    successor_node.predecessor_count -= 1;
                    if successor_node.predecessor_count == 0 {
                        // If we find nodes without any other prerequisites, we add them to the queue.
                        independent_nodes_queue.push(Reverse(successor_name));
                    }
                }
            }
        }
    }

    /// Get the in-degree of the node with the given name.
    fn indegree(&self, name: &str) -> Option<usize> {
        self.nodes.get(name).map(|data| data.predecessor_count)
    }

    // Pre-condition: self.nodes is non-empty.
    fn find_next_node(&mut self, frontier: &mut BinaryHeap<Reverse<&'input str>>) -> &'input str {
        // If there are no nodes of in-degree zero but there are still
        // un-visited nodes in the graph, then there must be a cycle.
        // We need to find the cycle, display it, and then break the
        // cycle.
        //
        // A cycle is guaranteed to be of length at least two. We break
        // the cycle by deleting an arbitrary edge (the first). That is
        // not necessarily the optimal thing, but it should be enough to
        // continue making progress in the graph traversal.
        //
        // It is possible that deleting the edge does not actually
        // result in the target node having in-degree zero, so we repeat
        // the process until such a node appears.
        loop {
            match frontier.pop() {
                None => self.find_and_break_cycle(frontier),
                Some(Reverse(v)) => return v,
            }
        }
    }

    fn find_and_break_cycle(&mut self, frontier: &mut BinaryHeap<Reverse<&'input str>>) {
        let cycle = self.detect_cycle();
        show!(TsortError::Loop(self.name.clone()));
        for node in &cycle {
            show!(TsortError::LoopNode((*node).to_string()));
        }
        let u = cycle[0];
        let v = cycle[1];
        self.remove_edge(u, v);
        if self.indegree(v).unwrap() == 0 {
            frontier.push(Reverse(v));
        }
    }

    fn detect_cycle(&self) -> Vec<&'input str> {
        // Sort the nodes just to make this function deterministic.
        let mut nodes = Vec::new();
        for node in self.nodes.keys() {
            nodes.push(node);
        }
        nodes.sort_unstable();

        let mut visited = HashSet::new();
        let mut stack = Vec::with_capacity(self.nodes.len());
        let mut stack_set = HashSet::with_capacity(self.nodes.len());
        for node in nodes {
            if !visited.contains(node) && self.dfs(node, &mut visited, &mut stack, &mut stack_set) {
                return stack;
            }
        }
        unreachable!();
    }

    fn dfs(
        &self,
        node: &'input str,
        visited: &mut HashSet<&'input str>,
        stack: &mut Vec<&'input str>,
        stack_set: &mut HashSet<&'input str>,
    ) -> bool {
        if stack_set.contains(&node) {
            return true;
        }
        if visited.contains(&node) {
            return false;
        }

        visited.insert(node);
        stack.push(node);
        stack_set.insert(node);

        if let Some(successor_names) = self.nodes.get(node).map(|n| &n.successor_names) {
            for &successor in successor_names {
                if self.dfs(successor, visited, stack, stack_set) {
                    return true;
                }
            }
        }

        stack.pop();
        stack_set.remove(node);
        false
    }
}
