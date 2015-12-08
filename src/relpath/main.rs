extern crate uu_relpath;

fn main() {
    std::process::exit(uu_relpath::uumain(std::env::args().collect()));
}
