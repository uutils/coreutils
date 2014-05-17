use std::io;
use std::io::process::Command;

static PROG: &'static str = "build/truncate";
static TFILE1: &'static str = "truncate_test_1";
static TFILE2: &'static str = "truncate_test_2";

fn make_file(name: &str) -> io::File {
    match io::File::create(&Path::new(name)) {
        Ok(f) => f,
        Err(_) => fail!()
    }
}

#[test]
fn test_increase_file_size() {
    let mut file = make_file(TFILE1);
    if !Command::new(PROG).args(["-s", "+5K", TFILE1]).status().unwrap().success() {
        fail!();
    }
    file.seek(0, io::SeekEnd).unwrap();
    if file.tell().unwrap() != 5 * 1024 {
        fail!();
    }
    io::fs::unlink(&Path::new(TFILE1)).unwrap();
}

#[test]
fn test_decrease_file_size() {
    let mut file = make_file(TFILE2);
    file.write(bytes!("1234567890")).unwrap();
    if !Command::new(PROG).args(["--size=-4", TFILE2]).status().unwrap().success() {
        fail!();
    }
    file.seek(0, io::SeekEnd).unwrap();
    if file.tell().unwrap() != 6 {
        println!("{}", file.tell());
        fail!();
    }
    io::fs::unlink(&Path::new(TFILE2)).unwrap();
}
