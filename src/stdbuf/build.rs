use std::process::Command;

fn main() {
    Command::new("make").output().expect("failed to execute make");
}
