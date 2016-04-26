#[macro_use]
mod common;
 
use common::util::*;
use std::path::Path;
use std::env;
use std::io::Write;
use std::fs::File;
use std::fs::remove_file;
 
static UTIL_NAME: &'static str = "od";
 
// octal dump of 'abcdefghijklmnopqrstuvwxyz\n'
static ALPHA_OUT: &'static str = "0000000    061141  062143  063145  064147  065151  066153  067155  070157\n0000020    071161  072163  073165  074167  075171  000012                \n0000033\n";
 
// XXX We could do a better job of ensuring that we have a fresh temp dir to ourself,
// not a general one ful of other proc's leftovers. 
 
// Test that od can read one file and dump with default format 
#[test]
fn test_file() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let temp = env::var("TMPDIR").unwrap_or_else(|_| env::var("TEMP").unwrap());
    let tmpdir = Path::new(&temp);
    let file = tmpdir.join("test");
     
    {
        let mut f = File::create(&file).unwrap();
        match f.write_all(b"abcdefghijklmnopqrstuvwxyz\n") {
            Err(_)  => panic!("Test setup failed - could not write file"),
            _ => {}
        }
    }
     
    let result = ucmd.arg(file.as_os_str()).run();
 
    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, ALPHA_OUT);
     
    let _ = remove_file(file);
}
 
// Test that od can read 2 files and concatenate the contents
#[test]
fn test_2files() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let temp = env::var("TMPDIR").unwrap_or_else(|_| env::var("TEMP").unwrap());
    let tmpdir = Path::new(&temp);
    let file1 = tmpdir.join("test1");
    let file2 = tmpdir.join("test2");
     
    for &(n,a) in [(1,"a"), (2,"b")].iter() {
        println!("number: {} letter:{}", n, a);
     }
         
    for &(path,data)in &[(&file1, "abcdefghijklmnop"),(&file2, "qrstuvwxyz\n")] {
        let mut f = File::create(&path).unwrap();
        match f.write_all(data.as_bytes()) {
            Err(_)  => panic!("Test setup failed - could not write file"),
            _ => {}
        }
    }
     
    let result = ucmd.arg(file1.as_os_str()).arg(file2.as_os_str()).run();
 
    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, ALPHA_OUT);
     
    let _ = remove_file(file1);
    let _ = remove_file(file2);
}

// Test that od gives non-0 exit val for filename that dosen't exist.   
#[test]
fn test_no_file() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let temp = env::var("TMPDIR").unwrap_or_else(|_| env::var("TEMP").unwrap());
    let tmpdir = Path::new(&temp);
    let file = tmpdir.join("}surely'none'would'thus'a'file'name");
     
    let result = ucmd.arg(file.as_os_str()).run();
     
    assert!(!result.success);
}

// Test that od reads from stdin instead of a file
#[test]
fn test_from_stdin() {
    let (_, mut ucmd) = testing(UTIL_NAME);

    let input = "abcdefghijklmnopqrstuvwxyz\n";
    let result = ucmd.run_piped_stdin(input.as_bytes());
    
    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, ALPHA_OUT);

}

// Test that od reads from stdin and also from files
#[test]
fn test_from_mixed() {
    let (_, mut ucmd) = testing(UTIL_NAME);

    let temp = env::var("TMPDIR").unwrap_or_else(|_| env::var("TEMP").unwrap());
    let tmpdir = Path::new(&temp);
    let file1 = tmpdir.join("test-1");
    let file3 = tmpdir.join("test-3");

    let (data1, data2, data3) = ("abcdefg","hijklmnop","qrstuvwxyz\n");
    for &(path,data)in &[(&file1, data1),(&file3, data3)] {
        let mut f = File::create(&path).unwrap();
        match f.write_all(data.as_bytes()) {
            Err(_)  => panic!("Test setup failed - could not write file"),
            _ => {}
        }
    }
   
    let result = ucmd.arg(file1.as_os_str()).arg("--").arg(file3.as_os_str()).run_piped_stdin(data2.as_bytes());
    
    assert_empty_stderr!(result);
    assert!(result.success);
    assert_eq!(result.stdout, ALPHA_OUT);

}
