extern crate uu_kill;

fn main() {
    std::process::exit(uu_kill::uumain(std::env::args().collect()));
}
