// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use clap::{crate_version, Arg, Command};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Write;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Read};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("tsort.md");
const USAGE: &str = help_usage!("tsort.md");

mod options {
    pub const FILE: &str = "file";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let input = matches
        .get_one::<String>(options::FILE)
        .expect("Value is required by clap");

    let mut stdin_buf;
    let mut file_buf;
    let mut reader = BufReader::new(if input == "-" {
        stdin_buf = stdin();
        &mut stdin_buf as &mut dyn Read
    } else {
        let path = Path::new(&input);
        if path.is_dir() {
            return Err(USimpleError::new(
                1,
                format!("{}: read error: Is a directory", input),
            ));
        }
        file_buf = File::open(path).map_err_context(|| input.to_string())?;
        &mut file_buf as &mut dyn Read
    });

    let mut g = Graph::new();
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(_) => {
                let tokens: Vec<String> = line.split_whitespace().map(|s| s.to_owned()).collect();
                if tokens.is_empty() {
                    break;
                }
                for ab in tokens.chunks(2) {
                    match ab.len() {
                        2 => g.add_edge(&ab[0], &ab[1]),
                        _ => {
                            return Err(USimpleError::new(
                                1,
                                format!(
                                    "{}: input contains an odd number of tokens",
                                    input.maybe_quote()
                                ),
                            ))
                        }
                    }
                }
            }
            _ => break,
        }
    }

    g.run_tsort();
    match g.result {
        Err(cycle) => {
            eprint!(
                "{}",
                cycle.iter().fold(
                    format!(
                        "{}: {}: input contains a loop:\n",
                        uucore::util_name(),
                        input
                    ),
                    |acc, node| {
                        let mut acc = acc;
                        writeln!(acc, "{}: {}", uucore::util_name(), node)
                            .expect("Failed to write to string");
                        acc
                    }
                )
            );
            println!("{}", cycle.join("\n"));
            Err(USimpleError::new(1, ""))
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

// Split off a slice from a VecDeque and return it as a Vec
// This is to avoid the need to convert the VecDeque to a Vec after splitting
// This function is inspired from the implementation of split_off in the standard library
fn split_off_as_vec<T>(deque: &mut VecDeque<T>, at: usize) -> Vec<T>
where
    T: Clone,
{
    assert!(at <= deque.len(), "`at` out of bounds");

    let (first_half, second_half) = deque.as_slices(); // In Rust, the deque is implemented as a
                                                       // two Vec buffer, so we can the slices directly
    let first_len = first_half.len();
    if at < first_len {
        // `at` lies in the first half.
        [&first_half[at..], second_half].concat()
    } else {
        // `at` lies in the second half,
        second_half[at - first_len..].to_vec()
    }
}

// We use String as a representation of node here
// but using integer may improve performance.
struct Graph {
    in_edges: HashMap<String, HashSet<String>>,
    out_edges: HashMap<String, Vec<String>>,
    result: Result<Vec<String>, Vec<String>>, // Stores either the topological sort result or the cycle
}

impl Graph {
    fn new() -> Self {
        Self {
            in_edges: HashMap::new(),
            out_edges: HashMap::new(),
            result: Ok(Vec::new()),
        }
    }

    fn has_edge(&self, from: &str, to: &str) -> bool {
        self.out_edges
            .get(from)
            .map_or(false, |edges| edges.contains(&to.to_string()))
    }

    fn init_node(&mut self, n: &str) {
        self.in_edges.entry(n.to_string()).or_default();
        self.out_edges.entry(n.to_string()).or_default();
    }

    fn add_edge(&mut self, from: &str, to: &str) {
        if from != to && !self.has_edge(from, to) {
            self.init_node(to); // Ensure both nodes are initialized
            self.init_node(from);
            self.in_edges.get_mut(to).unwrap().insert(from.to_string());
            self.out_edges.get_mut(from).unwrap().push(to.to_string());
        }
    }

    fn run_tsort(&mut self) {
        let mut visited = HashSet::new();
        let mut stack = VecDeque::new();
        let mut result = Vec::new();
        let mut nodes: Vec<&String> = self.out_edges.keys().collect();
        nodes.sort_unstable();
        for node in nodes {
            if !visited.contains(node.as_str()) {
                if let Err(cycle) = self.dfs(node, &mut visited, &mut stack, &mut result) {
                    self.result = Err(cycle);
                    return;
                }
            }
        }

        result.reverse(); // Reverse to get the correct topological order
        self.result = Ok(result);
    }

    fn dfs(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        stack: &mut VecDeque<String>,
        result: &mut Vec<String>,
    ) -> Result<(), Vec<String>> {
        if let Some(pos) = stack.iter().position(|x| x == node) {
            // Detected a cycle, return Err with the cycle's nodes
            return Err(split_off_as_vec(stack, pos));
        }
        if visited.contains(node) {
            return Ok(());
        }
        stack.push_back(node.to_string());
        visited.insert(node.to_string());

        if let Some(neighbors) = self.out_edges.get(node) {
            for neighbor in neighbors {
                self.dfs(neighbor, visited, stack, result)?;
            }
        }
        stack.pop_back();
        result.push(node.to_string());

        Ok(())
    }
}
