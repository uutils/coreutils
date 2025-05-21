// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//spell-checker:ignore TAOCP indegree
use clap::{Arg, Command};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{UError, UResult};
use uucore::{format_usage, help_about, help_usage, show};

const ABOUT: &str = help_about!("tsort.md");
const USAGE: &str = help_usage!("tsort.md");

mod options {
    pub const FILE: &str = "file";
}

#[derive(Debug, Error)]
enum TsortError {
    /// The input file is actually a directory.
    #[error("{0}: read error: Is a directory")]
    IsDir(String),

    /// The number of tokens in the input data is odd.
    ///
    /// The list of edges must be even because each edge has two
    /// components: a source node and a target node.
    #[error("{input}: input contains an odd number of tokens", input = .0.maybe_quote())]
    NumTokensOdd(String),

    /// The graph contains a cycle.
    #[error("{0}: input contains a loop:")]
    Loop(String),

    /// A particular node in a cycle. (This is mainly used for printing.)
    #[error("{0}")]
    LoopNode(String),
}

impl UError for TsortError {}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let input = matches
        .get_one::<String>(options::FILE)
        .expect("Value is required by clap");

    let data = if input == "-" {
        let stdin = std::io::stdin();
        std::io::read_to_string(stdin)?
    } else {
        let path = Path::new(&input);
        if path.is_dir() {
            return Err(TsortError::IsDir(input.to_string()).into());
        }
        std::fs::read_to_string(path)?
    };

    // Create the directed graph from pairs of tokens in the input data.
    let mut g = Graph::new(input.clone());
    for ab in data.split_whitespace().collect::<Vec<&str>>().chunks(2) {
        match ab {
            [a, b] => g.add_edge(a, b),
            _ => return Err(TsortError::NumTokensOdd(input.to_string()).into()),
        }
    }

    g.run_tsort();
    Ok(())
}
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .override_usage(format_usage(USAGE))
        .about(ABOUT)
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE)
                .default_value("-")
                .hide(true)
                .value_hint(clap::ValueHint::FilePath),
        )
}

/// Find the element `x` in `vec` and remove it, returning its index.
fn remove<T>(vec: &mut Vec<T>, x: T) -> Option<usize>
where
    T: PartialEq,
{
    vec.iter().position(|item| *item == x).inspect(|i| {
        vec.remove(*i);
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
        // First, we find a node that have no prerequisites (independent nodes)
        // If no such node exists, then there is a cycle.
        let mut independent_nodes_queue: VecDeque<&'input str> = self
            .nodes
            .iter()
            .filter_map(|(&name, node)| {
                if node.predecessor_count == 0 {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();

        // To make sure the resulting ordering is deterministic we
        // need to order independent nodes.
        //
        // FIXME: this doesn't comply entirely with the GNU coreutils
        // implementation.
        independent_nodes_queue.make_contiguous().sort_unstable();

        while !self.nodes.is_empty() {
            // Get the next node (breaking any cycles necessary to do so).
            let v = self.find_next_node(&mut independent_nodes_queue);
            println!("{v}");
            if let Some(node_to_process) = self.nodes.remove(v) {
                for successor_name in node_to_process.successor_names {
                    let successor_node = self.nodes.get_mut(successor_name).unwrap();
                    successor_node.predecessor_count -= 1;
                    if successor_node.predecessor_count == 0 {
                        // if we find nodes without any other prerequisites, we add them to the queue.
                        independent_nodes_queue.push_back(successor_name);
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
    fn find_next_node(&mut self, frontier: &mut VecDeque<&'input str>) -> &'input str {
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
            match frontier.pop_front() {
                None => self.find_and_break_cycle(frontier),
                Some(v) => return v,
            }
        }
    }

    fn find_and_break_cycle(&mut self, frontier: &mut VecDeque<&'input str>) {
        let cycle = self.detect_cycle();
        show!(TsortError::Loop(self.name.clone()));
        for node in &cycle {
            show!(TsortError::LoopNode(node.to_string()));
        }
        let u = cycle[0];
        let v = cycle[1];
        self.remove_edge(u, v);
        if self.indegree(v).unwrap() == 0 {
            frontier.push_back(v);
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
        for node in nodes {
            if !visited.contains(node) && self.dfs(node, &mut visited, &mut stack) {
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
    ) -> bool {
        if stack.contains(&node) {
            return true;
        }
        if visited.contains(&node) {
            return false;
        }

        visited.insert(node);
        stack.push(node);

        if let Some(successor_names) = self.nodes.get(node).map(|n| &n.successor_names) {
            for &successor in successor_names {
                if self.dfs(successor, visited, stack) {
                    return true;
                }
            }
        }

        stack.pop();
        false
    }
}
