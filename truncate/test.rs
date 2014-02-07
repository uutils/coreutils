use std::{run, io};

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
    if !run::process_status(PROG, [~"-s", ~"+5K", TFILE1.to_owned()]).unwrap().success() {
        fail!();
    }
    file.seek(0, io::SeekEnd);
    if file.tell().unwrap() != 5 * 1024 {
        fail!();
    }
    io::fs::unlink(&Path::new(TFILE1));
}

#[test]
fn test_decrease_file_size() {
    let mut file = make_file(TFILE2);
    file.write(bytes!("1234567890"));
    if !run::process_status(PROG, [~"--size=-4", TFILE2.to_owned()]).unwrap().success() {
        fail!();
    }
    file.seek(0, io::SeekEnd);
    if file.tell().unwrap() != 6 {
        println!("{}", file.tell());
        fail!();
    }
    io::fs::unlink(&Path::new(TFILE2));
}
