extern crate uu_stdbuf;

fn main() {
    std::process::exit(uu_stdbuf::uumain(std::env::args().collect()));
}
