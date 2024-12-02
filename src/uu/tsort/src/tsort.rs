// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//spell-checker:ignore TAOCP
use clap::{crate_version, Arg, Command};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Write;
use std::fs::File;
use std::io::{stdin, BufReader, Read};
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
                format!("{input}: read error: Is a directory"),
            ));
        }
        file_buf = File::open(path).map_err_context(|| input.to_string())?;
        &mut file_buf as &mut dyn Read
    });

    let mut input_buffer = String::new();
    reader.read_to_string(&mut input_buffer)?;
    let mut g = Graph::default();

    for line in input_buffer.lines() {
        let tokens: Vec<_> = line.split_whitespace().collect();
        if tokens.is_empty() {
            break;
        }
        for ab in tokens.chunks(2) {
            match ab.len() {
                2 => g.add_edge(ab[0], ab[1]),
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

    match g.run_tsort() {
        Err(cycle) => {
            let mut error_message = format!(
                "{}: {}: input contains a loop:\n",
                uucore::util_name(),
                input
            );
            for node in &cycle {
                writeln!(error_message, "{}: {}", uucore::util_name(), node).unwrap();
            }
            eprint!("{}", error_message);
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

// We use String as a representation of node here
// but using integer may improve performance.

struct Node<'input> {
    successor_names: Vec<&'input str>,
    predecessor_count: usize,
}

impl<'input> Node<'input> {
    fn new() -> Self {
        Node {
            successor_names: Vec::new(),
            predecessor_count: 0,
        }
    }

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
        self.nodes.entry(name).or_insert_with(Node::new);
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
