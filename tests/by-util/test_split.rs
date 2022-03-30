//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
// spell-checker:ignore xzaaa sixhundredfiftyonebytes ninetyonebytes threebytes asciilowercase fghij klmno pqrst uvwxyz fivelines twohundredfortyonebytes onehundredlines nbbbb
extern crate rand;
extern crate regex;

use self::rand::{thread_rng, Rng};
use self::regex::Regex;
use crate::common::util::*;
use rand::SeedableRng;
#[cfg(not(windows))]
use std::env;
use std::path::Path;
use std::{
    fs::{read_dir, File},
    io::{BufWriter, Read, Write},
};

fn random_chars(n: usize) -> String {
    thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .map(char::from)
        .take(n)
        .collect::<String>()
}

struct Glob {
    directory: AtPath,
    regex: Regex,
}

impl Glob {
    fn new(at: &AtPath, directory: &str, regex: &str) -> Self {
        Self {
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
            data.extend(self.directory.read_bytes(name));
        }
        data
    }
}

/// File handle that user can add random bytes (line-formatted or not) to
struct RandomFile {
    inner: File,
}

impl RandomFile {
    /// Size of each line that's being generated
    const LINESIZE: usize = 32;

    /// `create()` file handle located at `at` / `name`
    fn new(at: &AtPath, name: &str) -> Self {
        Self {
            inner: File::create(&at.plus(name)).unwrap(),
        }
    }

    fn add_bytes(&mut self, bytes: usize) {
        // Note that just writing random characters isn't enough to cover all
        // cases. We need truly random bytes.
        let mut writer = BufWriter::new(&self.inner);

        // Seed the rng so as to avoid spurious test failures.
        let mut rng = rand::rngs::StdRng::seed_from_u64(123);
        let mut buffer = [0; 1024];
        let mut remaining_size = bytes;

        while remaining_size > 0 {
            let to_write = std::cmp::min(remaining_size, buffer.len());
            let buf = &mut buffer[..to_write];
            rng.fill(buf);
            writer.write_all(buf).unwrap();

            remaining_size -= to_write;
        }
    }

    /// Add n lines each of size `RandomFile::LINESIZE`
    fn add_lines(&mut self, lines: usize) {
        let mut n = lines;
        while n > 0 {
            writeln!(self.inner, "{}", random_chars(Self::LINESIZE)).unwrap();
            n -= 1;
        }
    }
}

#[test]
fn test_split_default() {
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "split_default";
    RandomFile::new(&at, name).add_lines(2000);
    ucmd.args(&[name]).succeeds();

    let glob = Glob::new(&at, ".", r"x[[:alpha:]][[:alpha:]]$");
    assert_eq!(glob.count(), 2);
    assert_eq!(glob.collate(), at.read_bytes(name));
}

#[test]
fn test_split_numeric_prefixed_chunks_by_bytes() {
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "split_num_prefixed_chunks_by_bytes";
    RandomFile::new(&at, name).add_bytes(10000);
    ucmd.args(&[
        "-d", // --numeric-suffixes
        "-b", // --bytes
        "1000", name, "a",
    ])
    .succeeds();

    let glob = Glob::new(&at, ".", r"a\d\d$");
    assert_eq!(glob.count(), 10);
    for filename in glob.collect() {
        assert_eq!(glob.directory.metadata(&filename).len(), 1000);
    }
    assert_eq!(glob.collate(), at.read_bytes(name));
}

#[test]
fn test_split_str_prefixed_chunks_by_bytes() {
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "split_str_prefixed_chunks_by_bytes";
    RandomFile::new(&at, name).add_bytes(10000);
    // Important that this is less than 1024 since that's our internal buffer
    // size. Good to test that we don't overshoot.
    ucmd.args(&["-b", "1000", name, "b"]).succeeds();

    let glob = Glob::new(&at, ".", r"b[[:alpha:]][[:alpha:]]$");
    assert_eq!(glob.count(), 10);
    for filename in glob.collect() {
        assert_eq!(glob.directory.metadata(&filename).len(), 1000);
    }
    assert_eq!(glob.collate(), at.read_bytes(name));
}

// This is designed to test what happens when the desired part size is not a
// multiple of the buffer size and we hopefully don't overshoot the desired part
// size.
#[test]
fn test_split_bytes_prime_part_size() {
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "test_split_bytes_prime_part_size";
    RandomFile::new(&at, name).add_bytes(10000);
    // 1753 is prime and greater than the buffer size, 1024.
    ucmd.args(&["-b", "1753", name, "b"]).succeeds();

    let glob = Glob::new(&at, ".", r"b[[:alpha:]][[:alpha:]]$");
    assert_eq!(glob.count(), 6);
    let mut fns = glob.collect();
    // glob.collect() is not guaranteed to return in sorted order, so we sort.
    fns.sort();
    #[allow(clippy::needless_range_loop)]
    for i in 0..5 {
        assert_eq!(glob.directory.metadata(&fns[i]).len(), 1753);
    }
    assert_eq!(glob.directory.metadata(&fns[5]).len(), 1235);
    assert_eq!(glob.collate(), at.read_bytes(name));
}

#[test]
fn test_split_num_prefixed_chunks_by_lines() {
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "split_num_prefixed_chunks_by_lines";
    RandomFile::new(&at, name).add_lines(10000);
    ucmd.args(&["-d", "-l", "1000", name, "c"]).succeeds();

    let glob = Glob::new(&at, ".", r"c\d\d$");
    assert_eq!(glob.count(), 10);
    assert_eq!(glob.collate(), at.read_bytes(name));
}

#[test]
fn test_split_str_prefixed_chunks_by_lines() {
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "split_str_prefixed_chunks_by_lines";
    RandomFile::new(&at, name).add_lines(10000);
    ucmd.args(&["-l", "1000", name, "d"]).succeeds();

    let glob = Glob::new(&at, ".", r"d[[:alpha:]][[:alpha:]]$");
    assert_eq!(glob.count(), 10);
    assert_eq!(glob.collate(), at.read_bytes(name));
}

#[test]
fn test_split_additional_suffix() {
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "split_additional_suffix";
    RandomFile::new(&at, name).add_lines(2000);
    ucmd.args(&["--additional-suffix", ".txt", name]).succeeds();

    let glob = Glob::new(&at, ".", r"x[[:alpha:]][[:alpha:]].txt$");
    assert_eq!(glob.count(), 2);
    assert_eq!(glob.collate(), at.read_bytes(name));
}

#[test]
fn test_additional_suffix_no_slash() {
    new_ucmd!()
        .args(&["--additional-suffix", "a/b"])
        .fails()
        .usage_error("invalid suffix 'a/b', contains directory separator");
}

// note: the test_filter* tests below are unix-only
// windows support has been waived for now because of the difficulty of getting
// the `cmd` call right
// see https://github.com/rust-lang/rust/issues/29494

#[test]
#[cfg(unix)]
fn test_filter() {
    // like `test_split_default()` but run a command before writing
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "filtered";
    let n_lines = 3;
    RandomFile::new(&at, name).add_lines(n_lines);

    // change all characters to 'i'
    ucmd.args(&["--filter=sed s/./i/g > $FILE", name])
        .succeeds();

    // assert all characters are 'i' / no character is not 'i'
    // (assert that command succeeded)
    let glob = Glob::new(&at, ".", r"x[[:alpha:]][[:alpha:]]$");
    assert!(
        glob.collate().iter().find(|&&c| {
            // is not i
            c != (b'i')
            // is not newline
            && c != (b'\n')
        }) == None
    );
}

#[test]
#[cfg(unix)]
fn test_filter_with_env_var_set() {
    // This test will ensure that if $FILE env var was set before running --filter, it'll stay that
    // way
    // implemented like `test_split_default()` but run a command before writing
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "filtered";
    let n_lines = 3;
    RandomFile::new(&at, name).add_lines(n_lines);

    let env_var_value = "some-value";
    env::set_var("FILE", &env_var_value);
    ucmd.args(&[format!("--filter={}", "cat > $FILE").as_str(), name])
        .succeeds();

    let glob = Glob::new(&at, ".", r"x[[:alpha:]][[:alpha:]]$");
    assert_eq!(glob.collate(), at.read_bytes(name));
    assert!(env::var("FILE").unwrap_or_else(|_| "var was unset".to_owned()) == env_var_value);
}

#[test]
#[cfg(unix)]
fn test_filter_command_fails() {
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "filter-will-fail";
    RandomFile::new(&at, name).add_lines(4);

    ucmd.args(&["--filter=/a/path/that/totally/does/not/exist", name])
        .fails();
}

#[test]
fn test_split_lines_number() {
    // Test if stdout/stderr for '--lines' option is correct
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("file");

    scene
        .ucmd()
        .args(&["--lines", "2", "file"])
        .succeeds()
        .no_stderr()
        .no_stdout();
    scene
        .ucmd()
        .args(&["--lines", "2fb", "file"])
        .fails()
        .code_is(1)
        .stderr_only("split: invalid number of lines: '2fb'");
}

#[test]
fn test_split_invalid_bytes_size() {
    new_ucmd!()
        .args(&["-b", "1024R"])
        .fails()
        .code_is(1)
        .stderr_only("split: invalid number of bytes: '1024R'");
    #[cfg(not(target_pointer_width = "128"))]
    new_ucmd!()
        .args(&["-b", "1Y"])
        .fails()
        .code_is(1)
        .stderr_only("split: invalid number of bytes: '1Y': Value too large for defined data type");
    #[cfg(target_pointer_width = "32")]
    {
        let sizes = ["1000G", "10T"];
        for size in &sizes {
            new_ucmd!().args(&["-b", size]).succeeds();
        }
    }
}

#[test]
fn test_split_chunks_num_chunks_oversized_32() {
    #[cfg(target_pointer_width = "32")]
    {
        let scene = TestScenario::new(util_name!());
        let at = &scene.fixtures;
        at.touch("file");
        scene
            .ucmd()
            .args(&["--number", "5000000000", "file"])
            .fails()
            .code_is(1)
            .stderr_only("split: Number of chunks too big");
    }
}

#[test]
fn test_split_stdin_num_chunks() {
    new_ucmd!()
        .args(&["--number=1"])
        .fails()
        .code_is(1)
        .stderr_only("split: -: cannot determine file size");
}

fn file_read(at: &AtPath, filename: &str) -> String {
    let mut s = String::new();
    at.open(filename).read_to_string(&mut s).unwrap();
    s
}

// TODO Use char::from_digit() in Rust v1.51.0 or later.
fn char_from_digit(n: usize) -> char {
    (b'a' + n as u8) as char
}

/// Test for the default suffix length behavior: dynamically increasing size.
#[test]
fn test_alphabetic_dynamic_suffix_length() {
    let (at, mut ucmd) = at_and_ucmd!();
    // Split into chunks of one byte each.
    //
    // The input file has (26^2) - 26 + 1 = 651 bytes. This is just
    // enough to force `split` to dynamically increase the length of
    // the filename for the very last chunk.
    //
    // We expect the output files to be named
    //
    //     xaa, xab, xac, ..., xyx, xyy, xyz, xzaaa
    //
    ucmd.args(&["-b", "1", "sixhundredfiftyonebytes.txt"])
        .succeeds();
    for i in 0..25 {
        for j in 0..26 {
            let filename = format!("x{}{}", char_from_digit(i), char_from_digit(j),);
            let contents = file_read(&at, &filename);
            assert_eq!(contents, "a");
        }
    }
    assert_eq!(file_read(&at, "xzaaa"), "a");
}

/// Test for the default suffix length behavior: dynamically increasing size.
#[test]
fn test_numeric_dynamic_suffix_length() {
    let (at, mut ucmd) = at_and_ucmd!();
    // Split into chunks of one byte each, use numbers instead of
    // letters as file suffixes.
    //
    // The input file has (10^2) - 10 + 1 = 91 bytes. This is just
    // enough to force `split` to dynamically increase the length of
    // the filename for the very last chunk.
    //
    //     x00, x01, x02, ..., x87, x88, x89, x9000
    //
    ucmd.args(&["-d", "-b", "1", "ninetyonebytes.txt"])
        .succeeds();
    for i in 0..90 {
        let filename = format!("x{:02}", i);
        let contents = file_read(&at, &filename);
        assert_eq!(contents, "a");
    }
    assert_eq!(file_read(&at, "x9000"), "a");
}

#[test]
fn test_hex_dynamic_suffix_length() {
    let (at, mut ucmd) = at_and_ucmd!();
    // Split into chunks of one byte each, use hexadecimal digits
    // instead of letters as file suffixes.
    //
    // The input file has (16^2) - 16 + 1 = 241 bytes. This is just
    // enough to force `split` to dynamically increase the length of
    // the filename for the very last chunk.
    //
    //     x00, x01, x02, ..., xed, xee, xef, xf000
    //
    ucmd.args(&["-x", "-b", "1", "twohundredfortyonebytes.txt"])
        .succeeds();
    for i in 0..240 {
        let filename = format!("x{:02x}", i);
        let contents = file_read(&at, &filename);
        assert_eq!(contents, "a");
    }
    assert_eq!(file_read(&at, "xf000"), "a");
}

#[test]
fn test_suffixes_exhausted() {
    new_ucmd!()
        .args(&["-b", "1", "-a", "1", "asciilowercase.txt"])
        .fails()
        .stderr_only("split: output file suffixes exhausted");
}

#[test]
fn test_verbose() {
    new_ucmd!()
        .args(&["-b", "5", "--verbose", "asciilowercase.txt"])
        .succeeds()
        .stdout_only(
            "creating file 'xaa'
creating file 'xab'
creating file 'xac'
creating file 'xad'
creating file 'xae'
creating file 'xaf'
",
        );
}

#[test]
fn test_number() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_read = |f| {
        let mut s = String::new();
        at.open(f).read_to_string(&mut s).unwrap();
        s
    };
    ucmd.args(&["-n", "5", "asciilowercase.txt"]).succeeds();
    assert_eq!(file_read("xaa"), "abcde");
    assert_eq!(file_read("xab"), "fghij");
    assert_eq!(file_read("xac"), "klmno");
    assert_eq!(file_read("xad"), "pqrst");
    assert_eq!(file_read("xae"), "uvwxyz\n");
}

#[test]
fn test_split_number_with_io_blksize() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_read = |f| {
        let mut s = String::new();
        at.open(f).read_to_string(&mut s).unwrap();
        s
    };
    ucmd.args(&["-n", "5", "asciilowercase.txt", "---io-blksize", "1024"])
        .succeeds();
    assert_eq!(file_read("xaa"), "abcde");
    assert_eq!(file_read("xab"), "fghij");
    assert_eq!(file_read("xac"), "klmno");
    assert_eq!(file_read("xad"), "pqrst");
    assert_eq!(file_read("xae"), "uvwxyz\n");
}

#[test]
fn test_split_default_with_io_blksize() {
    let (at, mut ucmd) = at_and_ucmd!();
    let name = "split_default_with_io_blksize";
    RandomFile::new(&at, name).add_lines(2000);
    ucmd.args(&[name, "---io-blksize", "2M"]).succeeds();

    let glob = Glob::new(&at, ".", r"x[[:alpha:]][[:alpha:]]$");
    assert_eq!(glob.count(), 2);
    assert_eq!(glob.collate(), at.read_bytes(name));
}

#[test]
fn test_invalid_suffix_length() {
    new_ucmd!()
        .args(&["-a", "xyz"])
        .fails()
        .no_stdout()
        .stderr_contains("invalid suffix length: 'xyz'");
}

#[test]
fn test_include_newlines() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-l", "2", "fivelines.txt"]).succeeds();

    let mut s = String::new();
    at.open("xaa").read_to_string(&mut s).unwrap();
    assert_eq!(s, "1\n2\n");

    let mut s = String::new();
    at.open("xab").read_to_string(&mut s).unwrap();
    assert_eq!(s, "3\n4\n");

    let mut s = String::new();
    at.open("xac").read_to_string(&mut s).unwrap();
    assert_eq!(s, "5\n");
}

#[test]
fn test_allow_empty_files() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-n", "4", "threebytes.txt"])
        .succeeds()
        .no_stdout()
        .no_stderr();
    assert_eq!(at.read("xaa"), "a");
    assert_eq!(at.read("xab"), "b");
    assert_eq!(at.read("xac"), "c");
    assert_eq!(at.read("xad"), "");
}

#[test]
fn test_elide_empty_files() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-e", "-n", "4", "threebytes.txt"])
        .succeeds()
        .no_stdout()
        .no_stderr();
    assert_eq!(at.read("xaa"), "a");
    assert_eq!(at.read("xab"), "b");
    assert_eq!(at.read("xac"), "c");
    assert!(!at.plus("xad").exists());
}

#[test]
#[cfg(unix)]
fn test_elide_dev_null() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-e", "-n", "3", "/dev/null"])
        .succeeds()
        .no_stdout()
        .no_stderr();
    assert!(!at.plus("xaa").exists());
    assert!(!at.plus("xab").exists());
    assert!(!at.plus("xac").exists());
}

#[test]
fn test_lines() {
    let (at, mut ucmd) = at_and_ucmd!();

    let file_read = |f| {
        let mut s = String::new();
        at.open(f).read_to_string(&mut s).unwrap();
        s
    };

    // Split into two files without splitting up lines.
    ucmd.args(&["-n", "l/2", "fivelines.txt"]).succeeds();

    assert_eq!(file_read("xaa"), "1\n2\n3\n");
    assert_eq!(file_read("xab"), "4\n5\n");
}

#[test]
fn test_lines_kth() {
    new_ucmd!()
        .args(&["-n", "l/3/10", "onehundredlines.txt"])
        .succeeds()
        .stdout_only("20\n21\n22\n23\n24\n25\n26\n27\n28\n29\n");
}

#[test]
fn test_line_bytes() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-C", "8", "letters.txt"]).succeeds();
    assert_eq!(at.read("xaa"), "aaaaaaaa");
    assert_eq!(at.read("xab"), "a\nbbbb\n");
    assert_eq!(at.read("xac"), "cccc\ndd\n");
    assert_eq!(at.read("xad"), "ee\n");
}

#[test]
fn test_line_bytes_no_final_newline() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-C", "2"])
        .pipe_in("1\n2222\n3\n4")
        .succeeds()
        .no_stdout()
        .no_stderr();
    assert_eq!(at.read("xaa"), "1\n");
    assert_eq!(at.read("xab"), "22");
    assert_eq!(at.read("xac"), "22");
    assert_eq!(at.read("xad"), "\n");
    assert_eq!(at.read("xae"), "3\n");
    assert_eq!(at.read("xaf"), "4");
}

#[test]
fn test_line_bytes_no_empty_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-C", "1"])
        .pipe_in("1\n2222\n3\n4")
        .succeeds()
        .no_stdout()
        .no_stderr();
    assert_eq!(at.read("xaa"), "1");
    assert_eq!(at.read("xab"), "\n");
    assert_eq!(at.read("xac"), "2");
    assert_eq!(at.read("xad"), "2");
    assert_eq!(at.read("xae"), "2");
    assert_eq!(at.read("xaf"), "2");
    assert_eq!(at.read("xag"), "\n");
    assert_eq!(at.read("xah"), "3");
    assert_eq!(at.read("xai"), "\n");
    assert_eq!(at.read("xaj"), "4");
    assert!(!at.plus("xak").exists());
}
