use std::process::Command;

#[path = "../../mkmain.rs"]
mod mkmain;

fn main() {
    mkmain::main();

    Command::new("make").output().expect("failed to execute make");
}
