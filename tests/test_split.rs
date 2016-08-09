extern crate rand;
extern crate regex;

use std::fs::{File, read_dir};
use std::io::{Read, Write};
use std::path::Path;
use self::rand::{Rng, thread_rng};
use self::regex::Regex;
use common::util::*;

static UTIL_NAME: &'static str = "split";
fn at_and_ucmd() -> (AtPath, UCommand) {
    let ts = TestScenario::new(UTIL_NAME);
    let ucmd = ts.ucmd();
    (ts.fixtures, ucmd)
}

fn random_chars(n: usize) -> String {
    thread_rng().gen_ascii_chars().take(n).collect::<String>()
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

    fn collect(&self) -> Vec<String> {
        read_dir(Path::new(&self.directory.subdir))
            .unwrap()
            .filter_map(|entry| {
                let path = entry.unwrap().path();
                let name = self.directory.minus_as_string(path.as_path().to_str().unwrap_or(""));
                if self.regex.is_match(&name) {
                    Some(name)
                } else {
                    None
                }
            })
            .collect()
    }

    fn collate(&self) -> Vec<u8> {
        let mut files = self.collect();
        files.sort();
        let mut data: Vec<u8> = vec![];
        for name in files.iter() {
            data.extend(self.directory.read(name).into_bytes());
        }
        data
    }
}

struct RandomFile {
    inner: File,
}

impl RandomFile {
    fn new(at: &AtPath, name: &str) -> RandomFile {
        RandomFile { inner: File::create(&at.plus(name)).unwrap() }
    }

    fn add_bytes(&mut self, bytes: usize) {
        let chunk_size: usize = if bytes >= 1024 {
            1024
        } else {
            bytes
        };
        let mut n = bytes;
        while n > chunk_size {
            let _ = write!(self.inner, "{}", random_chars(chunk_size));
            n -= chunk_size;
        }
        let _ = write!(self.inner, "{}", random_chars(n));
    }

    fn add_lines(&mut self, lines: usize) {
        let line_size: usize = 32;
        let mut n = lines;
        while n > 0 {
            let _ = writeln!(self.inner, "{}", random_chars(line_size));
            n -= 1;
        }
    }
}

#[test]
fn test_split_default() {
    let (at, mut ucmd) = at_and_ucmd();
    let name = "split_default";
    let glob = Glob::new(&at, ".", r"x[:alpha:][:alpha:]$");
    RandomFile::new(&at, name).add_lines(2000);
    assert!(ucmd.args(&[name]).run().success);
    assert_eq!(glob.count(), 2);
    assert_eq!(glob.collate(), at.read(name).into_bytes());
}

#[test]
fn test_split_num_prefixed_chunks_by_bytes() {
    let (at, mut ucmd) = at_and_ucmd();
    let name = "split_num_prefixed_chunks_by_bytes";
    let glob = Glob::new(&at, ".", r"a\d\d$");
    RandomFile::new(&at, name).add_bytes(10000);
    assert!(ucmd.args(&["-d", "-b", "1000", name, "a"]).run().success);
    assert_eq!(glob.count(), 10);
    assert_eq!(glob.collate(), at.read(name).into_bytes());
}

#[test]
fn test_split_str_prefixed_chunks_by_bytes() {
    let (at, mut ucmd) = at_and_ucmd();
    let name = "split_str_prefixed_chunks_by_bytes";
    let glob = Glob::new(&at, ".", r"b[:alpha:][:alpha:]$");
    RandomFile::new(&at, name).add_bytes(10000);
    assert!(ucmd.args(&["-b", "1000", name, "b"]).run().success);
    assert_eq!(glob.count(), 10);
    assert_eq!(glob.collate(), at.read(name).into_bytes());
}

#[test]
fn test_split_num_prefixed_chunks_by_lines() {
    let (at, mut ucmd) = at_and_ucmd();
    let name = "split_num_prefixed_chunks_by_lines";
    let glob = Glob::new(&at, ".", r"c\d\d$");
    RandomFile::new(&at, name).add_lines(10000);
    assert!(ucmd.args(&["-d", "-l", "1000", name, "c"]).run().success);
    assert_eq!(glob.count(), 10);
    assert_eq!(glob.collate(), at.read(name).into_bytes());
}

#[test]
fn test_split_str_prefixed_chunks_by_lines() {
    let (at, mut ucmd) = at_and_ucmd();
    let name = "split_str_prefixed_chunks_by_lines";
    let glob = Glob::new(&at, ".", r"d[:alpha:][:alpha:]$");
    RandomFile::new(&at, name).add_lines(10000);
    assert!(ucmd.args(&["-l", "1000", name, "d"]).run().success);
    assert_eq!(glob.count(), 10);
    assert_eq!(glob.collate(), at.read(name).into_bytes());
}
