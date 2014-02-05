use std::{run, io};

static PROG: &'static str = "build/truncate";
static TESTNAME: &'static str = "THISISARANDOMFILENAME";

fn make_file() -> io::File {
    while Path::new(TESTNAME).exists() { io::timer::sleep(1000); }
    match io::File::create(&Path::new(TESTNAME)) {
        Ok(f) => f,
        Err(_) => fail!()
    }
}

#[test]
fn test_increase_file_size() {
    let mut file = make_file();
    if !run::process_status(PROG, [~"-s", ~"+5K", TESTNAME.to_owned()]).unwrap().success() {
        fail!();
    }
    file.seek(0, io::SeekEnd);
    if file.tell().unwrap() != 5 * 1024 {
        fail!();
    }
    io::fs::unlink(&Path::new(TESTNAME));
}

#[test]
fn test_decrease_file_size() {
    let mut file = make_file();
    file.write(bytes!("1234567890"));
    if !run::process_status(PROG, [~"--size=-4", TESTNAME.to_owned()]).unwrap().success() {
        fail!();
    }
    file.seek(0, io::SeekEnd);
    if file.tell().unwrap() != 6 {
        println!("{}", file.tell());
        fail!();
    }
    io::fs::unlink(&Path::new(TESTNAME));
}
