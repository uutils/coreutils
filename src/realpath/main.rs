extern crate uu_realpath;

fn main() {
    std::process::exit(uu_realpath::uumain(std::env::args().collect()));
}
