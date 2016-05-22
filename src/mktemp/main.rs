extern crate uu_mktemp;

fn main() {
    std::process::exit(uu_mktemp::uumain(std::env::args().collect()));
}
