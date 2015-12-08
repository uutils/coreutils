extern crate uu_chmod;

fn main() {
    std::process::exit(uu_chmod::uumain(std::env::args().collect()));
}
