// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//spell-checker:ignore TAOCP indegree
use clap::{Arg, Command};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
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
}

// Auxiliary struct, just for printing loop nodes via show! macro
#[derive(Debug, Error)]
#[error("{0}")]
struct LoopNode<'a>(&'a str);

impl UError for TsortError {}
impl UError for LoopNode<'_> {}

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
    let mut g = Graph::new(input.to_string_lossy().to_string());
    // Input is considered to be in the format
    // From1 To1 From2 To2 ...
    // with tokens separated by whitespaces
    let mut edge_tokens = data.split_whitespace();
    // Note: this is equivalent to iterating over edge_tokens.chunks(2)
    // but chunks() exists only for slices and would require unnecessary Vec allocation.
    // Itertools::chunks() is not used due to unnecessary overhead for internal RefCells
    loop {
        // Try take next pair of tokens
        let Some(from) = edge_tokens.next() else {
            // no more tokens -> end of input. Graph constructed
            break;
        };
        let Some(to) = edge_tokens.next() else {
            return Err(TsortError::NumTokensOdd(input.to_string_lossy().to_string()).into());
        };
        g.add_edge(from, to);
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum VisitedState {
    Opened,
    Closed,
}

impl<'input> Graph<'input> {
    fn new(name: String) -> Self {
        Self {
            name,
            nodes: HashMap::default(),
        }
    }

    fn add_edge(&mut self, from: &'input str, to: &'input str) {
        let from_node = self.nodes.entry(from).or_default();
        if from != to {
            from_node.add_successor(to);
            let to_node = self.nodes.entry(to).or_default();
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
                        // If we find nodes without any other prerequisites, we add them to the queue.
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
        for &node in &cycle {
            show!(LoopNode(node));
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
        let mut nodes: Vec<_> = self.nodes.keys().collect();
        nodes.sort_unstable();

        let mut visited = HashMap::new();
        let mut stack = Vec::with_capacity(self.nodes.len());
        for node in nodes {
            if self.dfs(node, &mut visited, &mut stack) {
                // last element in the stack appears twice: at the begin
                // and at the end of the loop
                let (loop_entry, _) = stack.pop().expect("loop is not empty");

                // skip the prefix which doesn't belong to the loop
                return stack
                    .into_iter()
                    .map(|(node, _)| node)
                    .skip_while(|&node| node != loop_entry)
                    .collect();
            }
        }
        unreachable!("detect_cycle is expected to be called only on graphs with cycles");
    }

    fn dfs<'a>(
        &'a self,
        node: &'input str,
        visited: &mut HashMap<&'input str, VisitedState>,
        stack: &mut Vec<(&'input str, &'a [&'input str])>,
    ) -> bool {
        stack.push((
            node,
            self.nodes.get(node).map_or(&[], |n| &n.successor_names),
        ));
        let state = *visited.entry(node).or_insert(VisitedState::Opened);

        if state == VisitedState::Closed {
            return false;
        }

        while let Some((node, pending_successors)) = stack.pop() {
            let Some((&next_node, pending)) = pending_successors.split_first() else {
                // no more pending successors in the list -> close the node
                visited.insert(node, VisitedState::Closed);
                continue;
            };

            // schedule processing for the pending part of successors for this node
            stack.push((node, pending));

            match visited.entry(next_node) {
                Entry::Vacant(v) => {
                    // It's a first time we enter this node
                    v.insert(VisitedState::Opened);
                    stack.push((
                        next_node,
                        self.nodes
                            .get(next_node)
                            .map_or(&[], |n| &n.successor_names),
                    ));
                }
                Entry::Occupied(o) => {
                    if *o.get() == VisitedState::Opened {
                        // we are entering the same opened node again -> loop found
                        // stack contains it
                        //
                        // But part of the stack may not be belonging to this loop
                        // push found node to the stack to be able to trace the beginning of the loop
                        stack.push((next_node, &[]));
                        return true;
                    }
                }
            }
        }

        false
    }
}
