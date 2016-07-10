extern crate uu_remove;

fn main() {
    std::process::exit(uu_remove::uumain(std::env::args().collect()));
}
