// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Benchmarks for hostname utility
//!
//! This benchmark tests the performance of hostname with large /etc/hosts files,
//! specifically targeting the DNS resolution functionality (`-i` flag).
//!
//! # Important Note on Large Hosts File Testing
//!
//! To properly test with large /etc/hosts files, NSS_WRAPPER library must be used:
//!
//! ```bash
//! LD_PRELOAD=/usr/lib/libnss_wrapper.so NSS_WRAPPER_HOSTS=/tmp/large_hosts \
//!   cargo bench --package uu_hostname hostname_ip_lookup
//! ```
//!
//! Without NSS_WRAPPER, the benchmark tests with the system's real /etc/hosts.

use divan::{Bencher, black_box};
use std::io::Write;
use uu_hostname::uumain;
use uucore::benchmark::run_util_function;

/// Generate a large hosts file with the specified number of entries
fn generate_hosts_file(entries: usize) -> Vec<u8> {
    let avg_line_size = 80;
    let mut data = Vec::with_capacity(entries * avg_line_size);
    let mut writer = std::io::BufWriter::new(&mut data);

    // Localhost entries
    writeln!(writer, "127.0.0.1   localhost localhost.localdomain").unwrap();
    writeln!(writer, "::1         localhost localhost.localdomain").unwrap();

    // Generate host entries
    let environments = ["prod", "staging", "dev", "test"];
    let regions = ["us-east-1", "us-west-2", "eu-west-1", "ap-southeast-1"];
    let roles = [
        "web", "db", "cache", "api", "worker", "lb", "monitor", "backup",
    ];

    for i in 2..=entries {
        let ip = match i % 5 {
            0 => format!("127.0.0.{}", i % 256),
            1 => format!("10.{}.{}.{}", (i / 256) % 256, (i / 16) % 16, i % 256),
            2 => format!("192.168.{}.{}", (i / 256) % 256, i % 256),
            3 => format!("172.16.{}.{}", (i / 256) % 256, i % 256),
            _ => format!("10.0.{}.{}", (i / 256) % 256, i % 256),
        };

        let role = roles[i % roles.len()];
        let env = environments[i % environments.len()];
        let region = regions[i % regions.len()];
        let fqdn = format!("{role}-{i:03}.{region}.{env}.example.com");
        let short = format!("{role}-{i:03}");

        writeln!(writer, "{ip:<15} {fqdn} {short}").unwrap();
    }

    writer.flush().unwrap();
    drop(writer);
    data
}

/// Benchmark hostname -i with large hosts file
///
/// # Important
/// Use NSS_WRAPPER to test with generated hosts file:
/// `LD_PRELOAD=/usr/lib/libnss_wrapper.so NSS_WRAPPER_HOSTS=/path/to/hosts cargo bench`
#[divan::bench(
    args = [100_000],
    name = "hostname_ip_lookup"
)]
fn bench_hostname_ip(bencher: Bencher, entries: usize) {
    let temp_dir = tempfile::tempdir().unwrap();
    let hosts_file = temp_dir.path().join("hosts");
    let hosts_data = generate_hosts_file(entries);
    std::fs::write(&hosts_file, &hosts_data).unwrap();

    let nss_wrapper_env = std::env::var("LD_PRELOAD").unwrap_or_default();
    let has_nss_wrapper = nss_wrapper_env.contains("nss_wrapper");

    if has_nss_wrapper {
        unsafe {
            std::env::set_var("NSS_WRAPPER_HOSTS", &hosts_file);
        }
    }

    bencher.bench(|| {
        let result = black_box(run_util_function(uumain, &["-i"]));
        assert_eq!(result, 0);
    });

    if has_nss_wrapper {
        unsafe {
            std::env::remove_var("NSS_WRAPPER_HOSTS");
        }
    }
}

/// Benchmark basic hostname display (baseline)
#[divan::bench(name = "hostname_basic")]
fn bench_hostname_basic(bencher: Bencher) {
    bencher.bench(|| {
        let result = black_box(run_util_function(uumain, &[]));
        assert_eq!(result, 0);
    });
}

/// Benchmark direct DNS lookup (Linux/macOS path)
#[cfg(not(any(target_os = "freebsd", target_os = "openbsd")))]
#[divan::bench(
    args = [100_000],
    name = "socket_addrs_direct"
)]
fn bench_socket_addrs_direct(bencher: Bencher, _entries: usize) {
    use std::net::ToSocketAddrs;

    let hostname = hostname::get()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let hostname_with_port = format!("{hostname}:1");

    bencher.bench(|| {
        let result: Result<Vec<_>, _> = hostname_with_port.to_socket_addrs().map(Iterator::collect);
        let _ = black_box(result);
    });
}

fn main() {
    divan::main();
}
