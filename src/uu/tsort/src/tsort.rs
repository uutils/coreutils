// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//spell-checker:ignore TAOCP
use clap::{crate_version, Arg, Command};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Display;
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{UError, UResult};
use uucore::{format_usage, help_about, help_usage, show};

const ABOUT: &str = help_about!("tsort.md");
const USAGE: &str = help_usage!("tsort.md");

mod options {
    pub const FILE: &str = "file";
}

#[derive(Debug)]
enum TsortError {
    /// The input file is actually a directory.
    IsDir(String),

    /// The number of tokens in the input data is odd.
    ///
    /// The list of edges must be even because each edge has two
    /// components: a source node and a target node.
    NumTokensOdd(String),

    /// The graph contains a cycle.
    Loop(String),

    /// A particular node in a cycle. (This is mainly used for printing.)
    LoopNode(String),
}

impl std::error::Error for TsortError {}

impl UError for TsortError {}

impl Display for TsortError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::IsDir(d) => write!(f, "{d}: read error: Is a directory"),
            Self::NumTokensOdd(i) => write!(
                f,
                "{}: input contains an odd number of tokens",
                i.maybe_quote()
            ),
            Self::Loop(i) => write!(f, "{i}: input contains a loop:"),
            Self::LoopNode(v) => write!(f, "{v}"),
        }
    }
}

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
    let mut g = Graph::default();
    for ab in data.split_whitespace().collect::<Vec<&str>>().chunks(2) {
        match ab {
            [a, b] => g.add_edge(a, b),
            _ => return Err(TsortError::NumTokensOdd(input.to_string()).into()),
        }
    }

    match g.run_tsort() {
        Err(cycle) => {
            show!(TsortError::Loop(input.to_string()));
            for node in &cycle {
                show!(TsortError::LoopNode(node.to_string()));
            }
            println!("{}", cycle.join("\n"));
            Ok(())
        }
        Ok(ordering) => {
            println!("{}", ordering.join("\n"));
            Ok(())
        }
    }
}
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
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
#[derive(Default)]
struct Graph<'input> {
    nodes: HashMap<&'input str, Node<'input>>,
}

impl<'input> Graph<'input> {
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
    /// Implementation of algorithm T from TAOCP (Don. Knuth), vol. 1.
    fn run_tsort(&mut self) -> Result<Vec<&'input str>, Vec<&'input str>> {
        let mut result = Vec::with_capacity(self.nodes.len());
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
        independent_nodes_queue.make_contiguous().sort_unstable(); // to make sure the resulting ordering is deterministic we need to order independent nodes
                                                                   // FIXME: this doesn't comply entirely with the GNU coreutils implementation.

        // we remove each independent node, from the graph, updating each successor predecessor_count variable as we do.
        while let Some(name_of_next_node_to_process) = independent_nodes_queue.pop_front() {
            result.push(name_of_next_node_to_process);
            if let Some(node_to_process) = self.nodes.remove(name_of_next_node_to_process) {
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

        // if the graph has no cycle (it's a dependency tree), the graph should be empty now, as all nodes have been deleted.
        if self.nodes.is_empty() {
            Ok(result)
        } else {
            // otherwise, we detect and show a cycle to the user (as the GNU coreutils implementation does)
            Err(self.detect_cycle())
        }
    }

    fn detect_cycle(&self) -> Vec<&'input str> {
        let mut visited = HashSet::new();
        let mut stack = Vec::with_capacity(self.nodes.len());
        for &node in self.nodes.keys() {
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
