// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use clap::{crate_version, Arg, Command};
use std::collections::{BTreeMap, BTreeSet};
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

    if !g.is_acyclic() {
        return Err(USimpleError::new(
            1,
            format!("{input}, input contains a loop:"),
        ));
    }

    for x in &g.result {
        println!("{x}");
    }

    Ok(())
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
struct Graph {
    in_edges: BTreeMap<String, BTreeSet<String>>,
    out_edges: BTreeMap<String, Vec<String>>,
    result: Vec<String>,
}

impl Graph {
    fn new() -> Self {
        Self::default()
    }

    fn has_node(&self, n: &str) -> bool {
        self.in_edges.contains_key(n)
    }

    fn has_edge(&self, from: &str, to: &str) -> bool {
        self.in_edges[to].contains(from)
    }

    fn init_node(&mut self, n: &str) {
        self.in_edges.insert(n.to_string(), BTreeSet::new());
        self.out_edges.insert(n.to_string(), vec![]);
    }

    fn add_edge(&mut self, from: &str, to: &str) {
        if !self.has_node(to) {
            self.init_node(to);
        }

        if !self.has_node(from) {
            self.init_node(from);
        }

        if from != to && !self.has_edge(from, to) {
            self.in_edges.get_mut(to).unwrap().insert(from.to_string());
            self.out_edges.get_mut(from).unwrap().push(to.to_string());
        }
    }

    // Kahn's algorithm
    // O(|V|+|E|)
    fn run_tsort(&mut self) {
        let mut start_nodes = vec![];
        for (n, edges) in &self.in_edges {
            if edges.is_empty() {
                start_nodes.push(n.clone());
            }
        }

        while !start_nodes.is_empty() {
            let n = start_nodes.remove(0);

            self.result.push(n.clone());

            let n_out_edges = self.out_edges.get_mut(&n).unwrap();
            #[allow(clippy::explicit_iter_loop)]
            for m in n_out_edges.iter() {
                let m_in_edges = self.in_edges.get_mut(m).unwrap();
                m_in_edges.remove(&n);

                // If m doesn't have other in-coming edges add it to start_nodes
                if m_in_edges.is_empty() {
                    start_nodes.push(m.clone());
                }
            }
            n_out_edges.clear();
        }
    }

    fn is_acyclic(&self) -> bool {
        self.out_edges.values().all(|edge| edge.is_empty())
    }
}
