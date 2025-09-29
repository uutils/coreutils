// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//spell-checker:ignore TAOCP indegree
use clap::{Arg, Command};
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

impl<'input> Graph<'input> {
    fn new(name: String) -> Graph<'input> {
        Self {
            name,
            nodes: HashMap::default(),
        }
    }

    fn add_edge(&mut self, from: &'input str, to: &'input str) {
        // Ensure both endpoints exist without holding long mutable borrows
        self.nodes.entry(from).or_default();
        if from != to {
            self.nodes.entry(to).or_default();
            // Check for duplicate edge using an immutable borrow
            let need_add = {
                let from_node_ro = self.nodes.get(from).unwrap();
                !from_node_ro.successor_names.contains(&to)
            };
            if need_add {
                // Now perform mutations via short, separate mutable borrows
                self.nodes.get_mut(from).unwrap().add_successor(to);
                self.nodes.get_mut(to).unwrap().predecessor_count += 1;
            }
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
        // Print a consistent loop header and each node in the detected cycle.
        // We report a minimal cycle subpath (no path prefix), matching GNU expectations
        // for cycle reporting while keeping runs deterministic.
        show!(TsortError::Loop(self.name.clone()));
        for node in &cycle {
            show!(TsortError::LoopNode((*node).to_string()));
        }
        // Remove the precise back-edge that closes the cycle: last -> first.
        // Rationale: removing the exact closing edge for the minimal cycle ensures
        // coherent follow-up behavior when multiple cycles exist. This strategy closely
        // matches the GNU example discussed in #8743 (B<->C first, then A->B->C->D->A).
        let u = *cycle.last().expect("cycle must be non-empty");
        let v = cycle[0];
        self.remove_edge(u, v);
        // If removing this edge unveils a new zero-indegree node, enqueue it so the main
        // topological loop can proceed without immediately triggering another cycle search.
        if self.indegree(v).unwrap() == 0 {
            frontier.push_back(v);
        }
    }

    fn detect_cycle(&self) -> Vec<&'input str> {
        // Delegate to CycleDetector which implements an iterative DFS to find and
        // reconstruct a true cycle deterministically.
        let detector = CycleDetector::new(self);
        detector
            .find_cycle()
            .expect("a cycle must exist when detect_cycle is called")
    }
}

/// Helper responsible for deterministic cycle detection using an explicit stack.
///
/// This separates concerns from Graph and enables easier swapping of algorithms
/// later (e.g., Tarjan/Kosaraju SCC) while keeping reporting behavior stable.
struct CycleDetector<'g, 'input> {
    graph: &'g Graph<'input>,
}

impl<'g, 'input> CycleDetector<'g, 'input> {
    fn new(graph: &'g Graph<'input>) -> Self {
        Self { graph }
    }

    /// Finds and returns a single cycle as a sequence of node names in which the last
    /// element connects back to the first (via the back-edge). Returns None if no cycle
    /// is found.
    ///
    /// Implementation notes:
    /// - We intern node names to integer IDs to speed up detection on large graphs.
    /// - We then run an iterative DFS over Vec-backed structures (visited, onstack, parent, adjacency).
    /// - When we find a back-edge (u -> v) with v on the stack, we reconstruct the minimal cycle
    ///   subpath [v, ..., u] and map IDs back to names for reporting.
    fn find_cycle(&self) -> Option<Vec<&'input str>> {
        use std::collections::HashMap;

        // 1) Collect and deterministically order node names for stable ID assignment.
        let mut names: Vec<&'input str> = self.graph.nodes.keys().copied().collect();
        names.sort_unstable();

        // 2) Build name -> id map and adjacency using IDs.
        let mut id_of: HashMap<&'input str, usize> = HashMap::with_capacity(names.len());
        for (i, &name) in names.iter().enumerate() {
            id_of.insert(name, i);
        }

        let n = names.len();
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (&name, node) in &self.graph.nodes {
            let u = id_of[&name];
            // successor_names may contain dupes; keep as-is since Graph.add_edge now dedups.
            for &s in &node.successor_names {
                if let Some(&v) = id_of.get(&s) {
                    adj[u].push(v);
                }
            }
        }

        // 3) Iterative DFS with explicit stack and parent tracking (Vec-based).
        let mut visited = vec![false; n];
        let mut onstack = vec![false; n];
        let mut parent: Vec<Option<usize>> = vec![None; n];

        for start in 0..n {
            if visited[start] {
                continue;
            }
            let mut stack: Vec<(usize, usize)> = Vec::new(); // (node_id, next_succ_idx)
            visited[start] = true;
            onstack[start] = true;
            stack.push((start, 0));

            while let Some((u, idx)) = stack.last_mut() {
                if *idx < adj[*u].len() {
                    let v = adj[*u][*idx];
                    *idx += 1;
                    if !visited[v] {
                        parent[v] = Some(*u);
                        visited[v] = true;
                        onstack[v] = true;
                        stack.push((v, 0));
                    } else if onstack[v] {
                        // Found back-edge u -> v. Reconstruct minimal cycle IDs [v, ..., u]
                        let mut cycle_ids: Vec<usize> = Vec::new();
                        cycle_ids.push(v);
                        let mut cur = *u;
                        let mut path: Vec<usize> = vec![cur];
                        while cur != v {
                            cur =
                                parent[cur].expect("parent must exist while reconstructing cycle");
                            path.push(cur);
                        }
                        path.pop();
                        path.reverse();
                        cycle_ids.extend(path);

                        // Map IDs back to names for reporting.
                        let cycle_names: Vec<&'input str> =
                            cycle_ids.into_iter().map(|id| names[id]).collect();
                        return Some(cycle_names);
                    }
                } else {
                    onstack[*u] = false;
                    stack.pop();
                }
            }
        }

        None
    }
}
