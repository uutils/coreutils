extern crate rand;
extern crate regex;

use self::rand::{thread_rng, Rng};
use self::regex::Regex;
use crate::common::util::*;
use std::fs::{read_dir, File};
use std::io::Write;
use std::path::Path;

fn random_chars(n: usize) -> String {
    thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(n)
        .collect::<String>()
}

struct Glob {
    directory: AtPath,
    regex: Regex,
}

impl Glob {
    fn new(at: &AtPath, directory: &str, regex: &str) -> Glob {
        Glob {
            directory: AtPath::new(Path::new(&at.plus_as_string(directory))),
            regex: Regex::new(regex).unwrap(),
        }
    }

    fn count(&self) -> usize {
        self.collect().len()
    }

    /// Get all files in `self.directory` that match `self.regex`
    fn collect(&self) -> Vec<String> {
        read_dir(Path::new(&self.directory.subdir))
            .unwrap()
            .filter_map(|entry| {
                let path = entry.unwrap().path();
                let name = self
                    .directory
                    .minus_as_string(path.as_path().to_str().unwrap_or(""));
                if self.regex.is_match(&name) {
                    Some(name)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Accumulate bytes of all files in `self.collect()`
    fn collate(&self) -> Vec<u8> {
        let mut files = self.collect();
        files.sort();
        let mut data: Vec<u8> = vec![];
        for name in &files {
            data.extend(self.directory.read(name).into_bytes());
        }
        data
    }
}

struct RandomFile {
    inner: File,
}

impl RandomFile {
    /// Size of each line that's being generated
    const LINESIZE: usize = 32;

    fn new(at: &AtPath, name: &str) -> RandomFile {
        RandomFile {
            inner: File::create(&at.plus(name)).unwrap(),
        }
    }

    fn add_bytes(&mut self, bytes: usize) {
        let chunk_size: usize = if bytes >= 1024 { 1024 } else { bytes };
        let mut n = bytes;
        while n > chunk_size {
            let _ = write!(self.inner, "{}", random_chars(chunk_size));
            n -= chunk_size;
        }
        let _ = write!(self.inner, "{}", random_chars(n));
    }

    /// Add n lines each of size `RandomFile::line_size`
    fn add_lines(&mut self, lines: usize) {
        let mut n = lines;
        while n > 0 {
            let _ = writeln!(self.inner, "{}", random_chars(RandomFile::LINESIZE));
            n -= 1;
        }
    }
}

#[test]
fn test_split_default() {
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "split_default";
    let glob = Glob::new(&at, ".", r"x[[:alpha:]][[:alpha:]]$");
    RandomFile::new(&at, name).add_lines(2000);
    ucmd.args(&[name]).succeeds();
    assert_eq!(glob.count(), 2);
    assert_eq!(glob.collate(), at.read(name).into_bytes());
}

#[test]
fn test_split_num_prefixed_chunks_by_bytes() {
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "split_num_prefixed_chunks_by_bytes";
    let glob = Glob::new(&at, ".", r"a\d\d$");
    RandomFile::new(&at, name).add_bytes(10000);
    ucmd.args(&["-d", "-b", "1000", name, "a"]).succeeds();
    assert_eq!(glob.count(), 10);
    assert_eq!(glob.collate(), at.read(name).into_bytes());
}

#[test]
fn test_split_str_prefixed_chunks_by_bytes() {
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "split_str_prefixed_chunks_by_bytes";
    let glob = Glob::new(&at, ".", r"b[[:alpha:]][[:alpha:]]$");
    RandomFile::new(&at, name).add_bytes(10000);
    ucmd.args(&["-b", "1000", name, "b"]).succeeds();
    assert_eq!(glob.count(), 10);
    assert_eq!(glob.collate(), at.read(name).into_bytes());
}

#[test]
fn test_split_num_prefixed_chunks_by_lines() {
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "split_num_prefixed_chunks_by_lines";
    let glob = Glob::new(&at, ".", r"c\d\d$");
    RandomFile::new(&at, name).add_lines(10000);
    ucmd.args(&["-d", "-l", "1000", name, "c"]).succeeds();
    assert_eq!(glob.count(), 10);
    assert_eq!(glob.collate(), at.read(name).into_bytes());
}

#[test]
fn test_split_str_prefixed_chunks_by_lines() {
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "split_str_prefixed_chunks_by_lines";
    let glob = Glob::new(&at, ".", r"d[[:alpha:]][[:alpha:]]$");
    RandomFile::new(&at, name).add_lines(10000);
    ucmd.args(&["-l", "1000", name, "d"]).succeeds();
    assert_eq!(glob.count(), 10);
    assert_eq!(glob.collate(), at.read(name).into_bytes());
}

#[test]
fn test_split_additional_suffix() {
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "split_additional_suffix";
    let glob = Glob::new(&at, ".", r"x[[:alpha:]][[:alpha:]].txt$");
    RandomFile::new(&at, name).add_lines(2000);
    ucmd.args(&["--additional-suffix", ".txt", name]).succeeds();
    assert_eq!(glob.count(), 2);
    assert_eq!(glob.collate(), at.read(name).into_bytes());
}

#[test]
#[cfg(unix)]
fn test_filter() {
    // like `test_split_default()` but run a command before writing
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "filtered";
    let glob = Glob::new(&at, ".", r"x[[:alpha:]][[:alpha:]]$");
    let n_lines = 2000;
    let n_bytes = 2000 * RandomFile::LINESIZE;
    RandomFile::new(&at, name).add_lines(n_lines);

    ucmd.args(&["--filter='sed s/./i/g > $FILE'", name])
        .succeeds();
    let single_char_vec = vec!['i' as u8; n_bytes as usize];
    assert_eq!(glob.collate(), single_char_vec);
}
