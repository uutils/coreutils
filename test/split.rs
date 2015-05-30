extern crate libc;
extern crate rand;
extern crate regex;

use std::fs::{File, read_dir, remove_file};
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;
use rand::{Rng, thread_rng};
use regex::Regex;
use util::*;

static PROGNAME: &'static str = "./split";

#[path = "common/util.rs"]
#[macro_use]
mod util;

fn random_chars(n: usize) -> String {
    thread_rng().gen_ascii_chars().take(n).collect::<String>()
}

struct Glob {
    directory: String,
    regex: Regex
}

impl Glob {
    fn new(directory: &str, regex: &str) -> Glob {
        Glob {
            directory: directory.to_string(),
            regex: Regex::new(regex).unwrap()
        }
    }

    fn count(&self) -> usize {
        self.collect().len()
    }

    fn collect(&self) -> Vec<String> {
        read_dir(Path::new(&self.directory)).unwrap().filter_map(|entry| {
            let path = entry.unwrap().path();
            let name = path.as_path().to_str().unwrap_or("");
            if self.regex.is_match(name) { Some(name.to_string()) } else { None }
        }).collect()
    }

    fn collate(&self) -> Vec<u8> {
        let mut files = self.collect();
        files.sort();
        let mut data: Vec<u8> = vec!();
        for name in files.iter() {
            data.extend(get_file_contents(name));
        }
        data
    }

    fn remove_all(&self) {
        for name in self.collect().iter() {
            let _ = remove_file(name);
        }
    }
}

struct RandomFile {
    inner: File
}

impl RandomFile {
    fn new(name: &str) -> RandomFile {
        RandomFile { inner: File::create(Path::new(name)).unwrap() }
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
    let name = "split_default";
    let glob = Glob::new(".", r"x[:alpha:][:alpha:]$");
    RandomFile::new(name).add_lines(2000);
    if !Command::new(PROGNAME).args(&[name]).status().unwrap().success() {
        panic!();
    }
    assert_eq!(glob.count(), 2);
    assert_eq!(glob.collate(), get_file_contents(name));
    glob.remove_all();
}

#[test]
fn test_split_num_prefixed_chunks_by_bytes() {
    let name = "split_num_prefixed_chunks_by_bytes";
    let glob = Glob::new(".", r"x\d\d$");
    RandomFile::new(name).add_bytes(10000);
    if !Command::new(PROGNAME).args(&["-d", "-b", "1000", name]).status().unwrap().success() {
        panic!();
    }
    assert_eq!(glob.count(), 10);
    assert_eq!(glob.collate(), get_file_contents(name));
    glob.remove_all();
}

#[test]
fn test_split_str_prefixed_chunks_by_bytes() {
    let name = "split_str_prefixed_chunks_by_bytes";
    let glob = Glob::new(".", r"x[:alpha:][:alpha:]$");
    RandomFile::new(name).add_bytes(10000);
    if !Command::new(PROGNAME).args(&["-b", "1000", name]).status().unwrap().success() {
        panic!();
    }
    assert_eq!(glob.count(), 10);
    assert_eq!(glob.collate(), get_file_contents(name));
    glob.remove_all();
}

#[test]
fn test_split_num_prefixed_chunks_by_lines() {
    let name = "split_num_prefixed_chunks_by_lines";
    let glob = Glob::new(".", r"x\d\d$");
    RandomFile::new(name).add_lines(10000);
    if !Command::new(PROGNAME).args(&["-d", "-l", "1000", name]).status().unwrap().success() {
        panic!();
    }
    assert_eq!(glob.count(), 10);
    assert_eq!(glob.collate(), get_file_contents(name));
    glob.remove_all();
}

#[test]
fn test_split_str_prefixed_chunks_by_lines() {
    let name = "split_str_prefixed_chunks_by_lines";
    let glob = Glob::new(".", r"x[:alpha:][:alpha:]$");
    RandomFile::new(name).add_lines(10000);
    if !Command::new(PROGNAME).args(&["-l", "1000", name]).status().unwrap().success() {
        panic!();
    }
    assert_eq!(glob.count(), 10);
    assert_eq!(glob.collate(), get_file_contents(name));
    glob.remove_all();
}
