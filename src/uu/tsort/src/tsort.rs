// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore TAOCP indegree
// spell-checker:ignore (libs) interner

mod error;
mod interner;
mod parser;

use clap::{Arg, ArgAction, Command};
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::collections::hash_map::Entry;
use std::ffi::OsString;
use std::fmt;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult, USimpleError};
use uucore::{format_usage, show, translate};

use crate::error::{Error, ReadError};
use crate::interner::{ByteInterner, Sym};

mod options {
    pub const FILE: &str = "file";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let mut inputs = matches
        .get_many::<OsString>(options::FILE)
        .into_iter()
        .flatten();

    let input = inputs.next().expect("default value should be set by clap");

    if let Some(extra) = inputs.next() {
        return Err(USimpleError::new(
            1,
            translate!(
                "tsort-error-extra-operand",
                "operand" => extra.quote()
            ),
        ));
    }

    // Create the directed graph from pairs of tokens in the input data.
    let mut g = Graph::new(input.to_string_lossy().to_string());

    if input == "-" {
        process_input(io::stdin().lock(), &mut g)?;
    } else {
        let mut options: OpenOptions;
        // some platforms cannot catch this as read error. Needs additional cost by stat
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_SEQUENTIAL_SCAN;
            let input = std::path::Path::new(input);
            if input.is_dir() {
                return Err(
                    Error::Read(ReadError::IsDir(input.to_string_lossy().to_string())).into(),
                );
            }
            // advise the OS we will access the data sequentially if possible (windows)
            options = File::options()
                .custom_flags(FILE_FLAG_SEQUENTIAL_SCAN)
                .clone();
        }

        #[cfg(not(windows))]
        {
            options = File::options();
        }
        let file = options
            .read(true)
            .open(input)
            .map_err_context(|| input.maybe_quote().to_string())?;

        // advise the OS we will access the data sequentially if possible (unix)
        #[cfg(all(
            any(unix, target_os = "wasi"),
            not(any(
                target_vendor = "apple",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly",
                target_os = "espidf",
                target_os = "haiku",
                target_os = "horizon",
                target_os = "redox",
                target_os = "solaris",
                target_os = "vita",
            ))
        ))]
        let _ = rustix::fs::fadvise(&file, 0, None, rustix::fs::Advice::Sequential);

        let reader = BufReader::new(file);
        process_input(reader, &mut g)?;
    }

    g.run_tsort()?;
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new("tsort")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("tsort"))
        .override_usage(format_usage(&translate!("tsort-usage")))
        .about(translate!("tsort-about"))
        .infer_long_args(true)
        // no-op flag, needed for POSIX compatibility.
        .arg(
            Arg::new("warn")
                .short('w')
                .action(ArgAction::SetTrue)
                .hide(true),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .value_parser(clap::value_parser!(OsString))
                .value_hint(clap::ValueHint::FilePath)
                .default_value("-")
                .num_args(1..)
                .action(ArgAction::Append),
        )
}

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

impl UError for Error {}
impl UError for LoopNode<'_> {}

fn process_input<R: BufRead>(reader: R, graph: &mut Graph) -> Result<(), Error> {
    let mut pending: Option<Sym> = None;

    // Input is considered to be in the format
    // From1 To1 From2 To2 ...
    // with tokens separated by whitespaces (<SPACE>, \t, or \n).
    //
    // Tokens are kept as raw bytes so invalid UTF-8 can be preserved.

    let result = parser::for_each_token(reader, |token| {
        let token_sym = graph.interner.get_or_intern(token);

        if let Some(from) = pending.take() {
            graph.add_edge(from, token_sym);
        } else {
            pending = Some(token_sym);
        }
    });

    if let Err(e) = result {
        if e.kind() == io::ErrorKind::IsADirectory {
            return Err(ReadError::IsDir(graph.name()).into());
        }
        return Err(ReadError::Io(e).into());
    }

    if pending.is_some() {
        return Err(ReadError::NumTokensOdd(graph.name()).into());
    }
    graph.interner.finish_interning();
    Ok(())
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
    fn add_successor(&mut self, successor_name: Sym) {
        self.successor_tokens.push(successor_name);
    }
}

struct Graph {
    name: String,
    nodes: FxHashMap<Sym, Node>,
    interner: ByteInterner,
}

impl Graph {
    fn new(name: String) -> Self {
        Self {
            name,
            nodes: FxHashMap::default(),
            interner: ByteInterner::default(),
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn get_node_name(&self, node_sym: Sym) -> &[u8] {
        self.interner
            .resolve(node_sym)
            .expect("symbol should be interned")
    }

    fn add_edge(&mut self, from: Sym, to: Sym) {
        let from_node = self.nodes.entry(from).or_default();

        if from != to {
            from_node.add_successor(to);

            let to_node = self.nodes.entry(to).or_default();
            to_node.predecessor_count += 1;
        }
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
    fn run_tsort(&mut self) -> Result<(), Error> {
        let mut independent_nodes_queue: VecDeque<Sym> = self
            .nodes
            .iter()
            .filter_map(|(&sym, node)| {
                if node.predecessor_count == 0 {
                    Some(sym)
                } else {
                    None
                }
            })
            .collect();

        // Sort by name for deterministic output.
        independent_nodes_queue
            .make_contiguous()
            .sort_unstable_by(|a, b| self.get_node_name(*a).cmp(self.get_node_name(*b)));

        let mut out = BufWriter::new(io::stdout().lock());

        while !self.nodes.is_empty() {
            let v = self.find_next_node(&mut independent_nodes_queue);

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
                        independent_nodes_queue.push_back(successor_name);
                    }
                }
            }
        }

        out.flush().map_err(Error::Write)?;
        Ok(())
    }

    pub fn indegree(&self, sym: Sym) -> Option<usize> {
        self.nodes.get(&sym).map(|data| data.predecessor_count)
    }

    fn find_next_node(&mut self, frontier: &mut VecDeque<Sym>) -> Sym {
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
                None => self.find_and_break_cycle(frontier),
                Some(v) => return v,
            }
        }
    }

    fn find_and_break_cycle(&mut self, frontier: &mut VecDeque<Sym>) {
        let cycle = self.detect_cycle();

        show!(Error::Loop(self.name()));

        for &sym in &cycle {
            show!(LoopNode(self.get_node_name(sym)));
        }

        let u = *cycle.last().expect("cycle must be non-empty");
        let v = cycle[0];

        self.remove_edge(u, v);

        if self.indegree(v).expect("node is part of the graph") == 0 {
            frontier.push_back(v);
        }
    }

    fn detect_cycle(&self) -> Vec<Sym> {
        // Sort by name for deterministic output.
        let mut nodes: Vec<_> = self.nodes.keys().copied().collect();

        nodes.sort_unstable_by(|a, b| self.get_node_name(*a).cmp(self.get_node_name(*b)));

        let mut visited = FxHashMap::default();
        let mut stack = Vec::with_capacity(self.nodes.len());

        for &node in &nodes {
            if self.dfs(node, &mut visited, &mut stack) {
                let (loop_entry, _) = stack.pop().expect("loop is not empty");

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
        node: Sym,
        visited: &mut FxHashMap<Sym, VisitedState>,
        stack: &mut Vec<(Sym, &'a [Sym])>,
    ) -> bool {
        stack.push((
            node,
            self.nodes
                .get(&node)
                .map_or(&[], |n: &Node| &n.successor_tokens),
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
                        // We have found a node that was already visited by another
                        // iteration => loop completed. The stack may contain
                        // unrelated nodes. This allows narrowing the loop down.
                        stack.push((next_node, &[]));
                        return true;
                    }
                }
            }
        }

        false
    }
}
