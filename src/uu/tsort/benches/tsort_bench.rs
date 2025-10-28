// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use divan::{Bencher, black_box};
use uu_tsort::uumain;
use uucore::benchmark::{run_util_function, setup_test_file};

/// Generate topological sort test data - linear chain
fn generate_linear_chain(num_nodes: usize) -> Vec<u8> {
    let mut data = Vec::new();

    for i in 0..num_nodes.saturating_sub(1) {
        data.extend_from_slice(format!("node{} node{}\n", i, i + 1).as_bytes());
    }

    data
}

/// Generate a DAG with tree-like structure
fn generate_tree_dag(depth: usize, branching_factor: usize) -> Vec<u8> {
    let mut data = Vec::new();
    let mut node_id = 0;

    // Generate a tree-like DAG
    for level in 0..depth {
        let nodes_at_level = branching_factor.pow(level as u32);

        for parent in 0..nodes_at_level {
            let parent_id = node_id + parent;
            for child in 0..branching_factor {
                if level + 1 < depth {
                    let child_id = node_id + nodes_at_level + parent * branching_factor + child;
                    data.extend_from_slice(format!("node{parent_id} node{child_id}\n").as_bytes());
                }
            }
        }
        node_id += nodes_at_level;
    }

    data
}

/// Generate a more complex graph with cross-dependencies
fn generate_complex_dag(num_nodes: usize) -> Vec<u8> {
    let mut data = Vec::new();

    // Create a diamond-like pattern with multiple levels
    let levels = ((num_nodes as f64).sqrt() as usize).max(4);
    let nodes_per_level = num_nodes / levels;

    for level in 0..levels - 1 {
        let start_current = level * nodes_per_level;
        let start_next = (level + 1) * nodes_per_level;
        let end_current = ((level + 1) * nodes_per_level).min(num_nodes);
        let end_next = ((level + 2) * nodes_per_level).min(num_nodes);

        for i in start_current..end_current {
            // Each node connects to 1-3 nodes in the next level
            let connections = ((i % 3) + 1).min(end_next - start_next);
            for j in 0..connections {
                let target = start_next + ((i + j) % (end_next - start_next));
                data.extend_from_slice(format!("node{i} node{target}\n").as_bytes());
            }
        }
    }

    data
}

/// Generate a random-like DAG that stresses the algorithm
fn generate_wide_dag(num_nodes: usize) -> Vec<u8> {
    let mut data = Vec::new();

    // Create many parallel chains that occasionally merge
    let num_chains = (num_nodes / 50).max(5);
    let chain_length = num_nodes / num_chains;

    for chain in 0..num_chains {
        let chain_start = chain * chain_length;
        let chain_end = ((chain + 1) * chain_length).min(num_nodes);

        // Build the chain
        for i in chain_start..chain_end.saturating_sub(1) {
            data.extend_from_slice(
                format!(
                    "chain{}_{} chain{}_{}\n",
                    chain,
                    i - chain_start,
                    chain,
                    i + 1 - chain_start
                )
                .as_bytes(),
            );
        }

        // Occasionally connect chains
        if chain > 0 && chain % 3 == 0 {
            let prev_chain = chain - 1;
            let prev_end = (prev_chain * chain_length + chain_length / 2).min(num_nodes - 1);
            let curr_mid = chain_start + chain_length / 4;
            data.extend_from_slice(
                format!(
                    "chain{}_{} chain{}_{}\n",
                    prev_chain,
                    prev_end - prev_chain * chain_length,
                    chain,
                    curr_mid - chain_start
                )
                .as_bytes(),
            );
        }
    }

    data
}

/// Generate DAG data for input parsing stress tests
fn generate_input_parsing_heavy(num_edges: usize) -> Vec<u8> {
    // Create a scenario with many edges but relatively few unique nodes
    // This stresses the input parsing and graph construction optimizations
    let num_unique_nodes = (num_edges as f64).sqrt() as usize;
    let mut data = Vec::new();

    for i in 0..num_edges {
        let from = i % num_unique_nodes;
        let to = (i / num_unique_nodes) % num_unique_nodes;
        if from != to {
            data.extend_from_slice(format!("n{from} n{to}\n").as_bytes());
        }
    }

    data
}

/// Benchmark linear chain graphs of different sizes
/// This tests the performance improvements mentioned in PR #8694
#[divan::bench(args = [1_000_000])]
fn tsort_linear_chain(bencher: Bencher, num_nodes: usize) {
    let data = generate_linear_chain(num_nodes);
    let file_path = setup_test_file(&data);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path_str]));
    });
}

/// Benchmark tree-like DAG structures
#[divan::bench(args = [(10, 3)])]
fn tsort_tree_dag(bencher: Bencher, (depth, branching): (usize, usize)) {
    let data = generate_tree_dag(depth, branching);
    let file_path = setup_test_file(&data);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path_str]));
    });
}

/// Benchmark complex DAG with cross-dependencies
#[divan::bench(args = [50_000])]
fn tsort_complex_dag(bencher: Bencher, num_nodes: usize) {
    let data = generate_complex_dag(num_nodes);
    let file_path = setup_test_file(&data);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path_str]));
    });
}

/// Benchmark wide DAG with many parallel chains
/// This should stress the hashmap optimizations from PR #8694
#[divan::bench(args = [100_000])]
fn tsort_wide_dag(bencher: Bencher, num_nodes: usize) {
    let data = generate_wide_dag(num_nodes);
    let file_path = setup_test_file(&data);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path_str]));
    });
}

/// Benchmark input parsing vs computation by using files with different edge densities
#[divan::bench(args = [5_000])]
fn tsort_input_parsing_heavy(bencher: Bencher, num_edges: usize) {
    let data = generate_input_parsing_heavy(num_edges);
    let file_path = setup_test_file(&data);
    let file_path_str = file_path.to_str().unwrap();

    bencher.bench(|| {
        black_box(run_util_function(uumain, &[file_path_str]));
    });
}

fn main() {
    divan::main();
}
