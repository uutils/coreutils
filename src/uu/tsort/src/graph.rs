// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore TAOCP indegree

use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::collections::hash_map::Entry;
use std::fmt;
use std::io::{self, BufWriter, Write};
use uucore::error::UError;
use uucore::show;

use crate::error::{AllocationError, Error};
use crate::interner::{ByteInterner, ByteInternerBuilder, Sym};
use crate::try_clone_str;

// Auxiliary struct, just for printing loop nodes via show! macro.
//
// Diagnostics go through Display, so invalid UTF-8 bytes are represented
// lossily here.
#[derive(Debug)]
struct LoopNode<'a>(&'a [u8]);

impl fmt::Display for LoopNode<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(self.0))
    }
}

impl std::error::Error for LoopNode<'_> {}
impl UError for LoopNode<'_> {}

/// Find the element `x` in `vec` and remove it, returning its index.
fn remove<T>(vec: &mut Vec<T>, x: T) -> Option<usize>
where
    T: PartialEq,
{
    vec.iter().position(|item| *item == x).inspect(|i| {
        vec.remove(*i);
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VisitedState {
    Opened,
    Closed,
}

#[derive(Default)]
struct Node {
    successor_tokens: Vec<Sym>,
    predecessor_count: usize,
}

impl Node {
    fn add_successor(&mut self, successor_name: Sym) -> Result<(), AllocationError> {
        self.successor_tokens.try_reserve(1)?;
        self.successor_tokens.push(successor_name);
        Ok(())
    }
}

pub(crate) struct GraphBuilder {
    name: String,
    nodes: FxHashMap<Sym, Node>,
    interner: ByteInternerBuilder,
}

impl GraphBuilder {
    pub(crate) fn new(name: String) -> Self {
        Self {
            name,
            nodes: FxHashMap::default(),
            interner: ByteInternerBuilder::default(),
        }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub(crate) fn intern(&mut self, value: &[u8]) -> Result<Sym, AllocationError> {
        self.interner.get_or_intern(value)
    }

    pub(crate) fn add_edge(&mut self, from: Sym, to: Sym) -> Result<(), AllocationError> {
        // Reserve only for keys that are actually absent. This avoids turning an
        // edge between existing nodes into a spurious capacity-overflow failure.
        let missing_from = !self.nodes.contains_key(&from);
        let missing_to = from != to && !self.nodes.contains_key(&to);
        let additional = usize::from(missing_from) + usize::from(missing_to);
        self.nodes.try_reserve(additional)?;

        let from_node = self.nodes.entry(from).or_default();

        if from != to {
            from_node.add_successor(to)?;

            let to_node = self.nodes.entry(to).or_default();
            to_node.predecessor_count += 1;
        }

        Ok(())
    }

    pub(crate) fn finish(self) -> Graph {
        Graph {
            name: self.name,
            nodes: self.nodes,
            interner: self.interner.finish(),
        }
    }
}

pub(crate) struct Graph {
    name: String,
    nodes: FxHashMap<Sym, Node>,
    interner: ByteInterner,
}

impl Graph {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_node_name(&self, node_sym: Sym) -> &[u8] {
        self.interner
            .resolve(node_sym)
            .expect("symbol should be interned")
    }

    fn remove_edge(&mut self, u: Sym, v: Sym) {
        remove(
            &mut self
                .nodes
                .get_mut(&u)
                .expect("node is part of the graph")
                .successor_tokens,
            v,
        );

        self.nodes
            .get_mut(&v)
            .expect("node is part of the graph")
            .predecessor_count -= 1;
    }

    /// Implementation of algorithm T from TAOCP (Don. Knuth), vol. 1.
    pub(crate) fn run_tsort(&mut self) -> Result<(), Error> {
        // A node is enqueued at most once, so reserving the number of graph nodes
        // covers the queue for the whole sort.
        let mut independent_nodes_queue = VecDeque::new();
        independent_nodes_queue
            .try_reserve(self.nodes.len())
            .map_err(AllocationError::from)?;

        for (&sym, node) in &self.nodes {
            if node.predecessor_count == 0 {
                independent_nodes_queue.push_back(sym);
            }
        }

        // Sort by name for deterministic output.
        independent_nodes_queue
            .make_contiguous()
            .sort_unstable_by(|a, b| self.get_node_name(*a).cmp(self.get_node_name(*b)));

        let mut out = BufWriter::new(io::stdout().lock());

        while !self.nodes.is_empty() {
            let v = self.find_next_node(&mut independent_nodes_queue)?;

            // Write the node exactly as it appeared in the input, followed by a newline
            out.write_all(self.get_node_name(v)).map_err(Error::Write)?;
            writeln!(out).map_err(Error::Write)?;

            if let Some(node_to_process) = self.nodes.remove(&v) {
                for successor_name in node_to_process.successor_tokens.into_iter().rev() {
                    // we reverse to match GNU tsort order
                    let successor_node = self
                        .nodes
                        .get_mut(&successor_name)
                        .expect("node is part of the graph");

                    successor_node.predecessor_count -= 1;

                    if successor_node.predecessor_count == 0 {
                        // The queue was reserved for every graph node above, and
                        // each node reaches in-degree zero at most once.
                        independent_nodes_queue.push_back(successor_name);
                    }
                }
            }
        }

        out.flush().map_err(Error::Write)?;
        Ok(())
    }

    fn indegree(&self, sym: Sym) -> Option<usize> {
        self.nodes.get(&sym).map(|data| data.predecessor_count)
    }

    fn find_next_node(&mut self, frontier: &mut VecDeque<Sym>) -> Result<Sym, Error> {
        // If there are no nodes of in-degree zero but there are still
        // un-visited nodes in the graph, then there must be a cycle.
        // We need to find the cycle, display it on stderr, and break it to go on.
        //
        // A cycle is guaranteed to be of length at least two. We break
        // the cycle by deleting an arbitrary edge (the first). That is
        // not necessarily the optimal thing, but it should be enough to
        // continue making progress in the graph traversal, and matches GNU tsort behavior.
        //
        // It is possible that deleting the edge does not actually
        // result in the target node having in-degree zero, so we repeat
        // the process until such a node appears.

        loop {
            match frontier.pop_front() {
                None => self.find_and_break_cycle(frontier)?,
                Some(v) => return Ok(v),
            }
        }
    }

    fn find_and_break_cycle(&mut self, frontier: &mut VecDeque<Sym>) -> Result<(), Error> {
        let cycle = self.detect_cycle()?;

        show!(Error::Loop(try_clone_str(self.name())?));

        for &sym in &cycle {
            show!(LoopNode(self.get_node_name(sym)));
        }

        let u = *cycle.last().expect("cycle must be non-empty");
        let v = cycle[0];

        self.remove_edge(u, v);

        if self.indegree(v).expect("node is part of the graph") == 0 {
            // `frontier` was reserved for all nodes in run_tsort().
            frontier.push_back(v);
        }

        Ok(())
    }

    fn detect_cycle(&self) -> Result<Vec<Sym>, Error> {
        // All three work structures are bounded by the number of remaining
        // graph nodes, which is known at this point.
        let node_count = self.nodes.len();

        let mut nodes = Vec::new();
        nodes
            .try_reserve(node_count)
            .map_err(AllocationError::from)?;
        for &sym in self.nodes.keys() {
            nodes.push(sym);
        }

        // Sort by name for deterministic output.
        nodes.sort_unstable_by(|a, b| self.get_node_name(*a).cmp(self.get_node_name(*b)));

        let mut visited = FxHashMap::default();
        visited
            .try_reserve(node_count)
            .map_err(AllocationError::from)?;

        let mut stack = Vec::new();
        stack
            .try_reserve(node_count)
            .map_err(AllocationError::from)?;

        for &node in &nodes {
            if let Some(loop_entry) = self.dfs(node, &mut visited, &mut stack) {
                let start = stack
                    .iter()
                    .rposition(|(node, _)| *node == loop_entry)
                    .expect("loop entry must be on the DFS stack");

                let mut cycle = Vec::new();
                cycle
                    .try_reserve(stack.len() - start)
                    .map_err(AllocationError::from)?;

                for &(node, _) in &stack[start..] {
                    cycle.push(node);
                }

                return Ok(cycle);
            }
        }

        unreachable!("detect_cycle is expected to be called only on graphs with cycles");
    }

    fn dfs<'a>(
        &'a self,
        node: Sym,
        visited: &mut FxHashMap<Sym, VisitedState>,
        stack: &mut Vec<(Sym, &'a [Sym])>,
    ) -> Option<Sym> {
        stack.push((
            node,
            self.nodes
                .get(&node)
                .map_or(&[], |n: &Node| &n.successor_tokens),
        ));

        let state = *visited.entry(node).or_insert(VisitedState::Opened);

        if state == VisitedState::Closed {
            // Remove the frame we just added. Keeping closed frames around makes
            // the stack larger and complicates extracting the actual cycle.
            stack.pop();
            return None;
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
                    // first visit of the node
                    v.insert(VisitedState::Opened);

                    stack.push((
                        next_node,
                        self.nodes
                            .get(&next_node)
                            .map_or(&[], |n| &n.successor_tokens),
                    ));
                }

                Entry::Occupied(o) => {
                    if *o.get() == VisitedState::Opened {
                        return Some(next_node);
                    }
                }
            }
        }

        None
    }
}
