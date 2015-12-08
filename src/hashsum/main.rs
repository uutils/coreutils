extern crate uu_hashsum;

fn main() {
    std::process::exit(uu_hashsum::uumain(std::env::args().collect()));
}
