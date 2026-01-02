// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//spell-checker:ignore TAOCP indegree fadvise FADV
//spell-checker:ignore (libs) interner uclibc
use clap::{Arg, ArgAction, Command};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use string_interner::StringInterner;
use string_interner::backend::BucketBackend;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{UError, UResult, USimpleError};
use uucore::{format_usage, show, translate};

// short types for switching interning behavior on the fly.
type Sym = string_interner::symbol::SymbolUsize;
type Interner = StringInterner<BucketBackend<Sym>>;

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

    let input = match (inputs.next(), inputs.next()) {
        (None, _) => {
            return Err(USimpleError::new(
                1,
                translate!("tsort-error-at-least-one-input"),
            ));
        }
        (Some(input), None) => input,
        (Some(_), Some(extra)) => {
            return Err(USimpleError::new(
                1,
                translate!(
                    "tsort-error-extra-operand",
                    "operand" => extra.quote(),
                    "util" => uucore::util_name()
                ),
            ));
        }
    };
    let file: File;
    // Create the directed graph from pairs of tokens in the input data.
    let mut g = Graph::new(input.to_string_lossy().to_string());
    if input == "-" {
        process_input(io::stdin().lock(), &mut g)?;
    } else {
        // Windows reports a permission denied error when trying to read a directory.
        // So we check manually beforehand. On other systems, we avoid this extra check for performance.
        #[cfg(windows)]
        {
            use std::path::Path;

            let path = Path::new(input);
            if path.is_dir() {
                return Err(TsortError::IsDir(input.to_string_lossy().to_string()).into());
            }

            file = File::open(path)?;
        }
        #[cfg(not(windows))]
        {
            file = File::open(input)?;

            // advise the OS we will access the data sequentially if available.
            #[cfg(any(
                target_os = "linux",
                target_os = "android",
                target_os = "fuchsia",
                target_os = "wasi",
                target_env = "uclibc",
                target_os = "freebsd",
            ))]
            {
                use nix::fcntl::{PosixFadviseAdvice, posix_fadvise};
                use std::os::unix::io::AsFd;

                posix_fadvise(
                    file.as_fd(),
                    0, // offset 0 => from the start of the file
                    0, // length 0 => for the whole file
                    PosixFadviseAdvice::POSIX_FADV_SEQUENTIAL,
                )
                .ok();
            }
        }
        let reader = BufReader::new(file);
        process_input(reader, &mut g)?;
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

#[derive(Debug, Error)]
enum TsortError {
    /// The input file is actually a directory.
    #[error("{input}: {message}", input = .0.maybe_quote(), message = translate!("tsort-error-is-dir"))]
    IsDir(String),

    /// The number of tokens in the input data is odd.
    ///
    /// The length of the list of edges must be even because each edge has two
    /// components: a source node and a target node.
    #[error("{input}: {message}", input = .0.maybe_quote(), message = translate!("tsort-error-odd"))]
    NumTokensOdd(String),

    /// The graph contains a cycle.
    #[error("{input}: {message}", input = .0, message = translate!("tsort-error-loop"))]
    Loop(String),

    /// Wrapper for bubbling up IO errors
    #[error("{0}")]
    IO(#[from] std::io::Error),
}

// Auxiliary struct, just for printing loop nodes via show! macro
#[derive(Debug, Error)]
#[error("{0}")]
struct LoopNode<'a>(&'a str);

impl UError for TsortError {}
impl UError for LoopNode<'_> {}

fn process_input<R: BufRead>(reader: R, graph: &mut Graph) -> Result<(), TsortError> {
    let mut pending: Option<Sym> = None;

    // Input is considered to be in the format
    // From1 To1 From2 To2 ...
    // with tokens separated by whitespaces

    for line in reader.lines() {
        let line = line.map_err(|e| {
            if e.kind() == io::ErrorKind::IsADirectory {
                TsortError::IsDir(graph.name())
            } else {
                e.into()
            }
        })?;
        for token in line.split_whitespace() {
            // Intern the token and get a Sym
            let token_sym = graph.interner.get_or_intern(token);

            if let Some(from) = pending.take() {
                graph.add_edge(from, token_sym);
            } else {
                pending = Some(token_sym);
            }
        }
    }
    if pending.is_some() {
        return Err(TsortError::NumTokensOdd(graph.name()));
    }

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
    name_sym: Sym,
    nodes: HashMap<Sym, Node>,
    interner: Interner,
}

impl Graph {
    fn new(name: String) -> Self {
        let mut interner = Interner::new();
        let name_sym = interner.get_or_intern(name);
        Self {
            name_sym,
            interner,
            nodes: HashMap::default(),
        }
    }

    fn name(&self) -> String {
        //SAFETY: the name is interned during graph creation and stored as name_sym.
        // gives much better performance on lookup.
        unsafe { self.interner.resolve_unchecked(self.name_sym).to_owned() }
    }
    fn get_node_name(&self, node_sym: Sym) -> &str {
        //SAFETY: the only way to get a Sym is by manipulating an interned string.
        // gives much better performance on lookup.

        unsafe { self.interner.resolve_unchecked(node_sym) }
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
    fn run_tsort(&mut self) {
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

        // Sort by resolved string for deterministic output
        independent_nodes_queue
            .make_contiguous()
            .sort_unstable_by(|a, b| self.get_node_name(*a).cmp(self.get_node_name(*b)));

        while !self.nodes.is_empty() {
            let v = self.find_next_node(&mut independent_nodes_queue);
            println!("{}", self.get_node_name(v));
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
        show!(TsortError::Loop(self.name()));
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
        // Sort by resolved string for deterministic output
        let mut nodes: Vec<_> = self.nodes.keys().copied().collect();
        nodes.sort_unstable_by(|a, b| self.get_node_name(*a).cmp(self.get_node_name(*b)));

        let mut visited = HashMap::new();
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
        visited: &mut HashMap<Sym, VisitedState>,
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
                        // We have found a node that was already visited by another iteration => loop completed
                        // the stack may contain unrelated nodes. This allows narrowing the loop down.
                        stack.push((next_node, &[]));
                        return true;
                    }
                }
            }
        }

        false
    }
}
