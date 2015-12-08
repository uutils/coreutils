extern crate uu_unexpand;

fn main() {
    std::process::exit(uu_unexpand::uumain(std::env::args().collect()));
}
