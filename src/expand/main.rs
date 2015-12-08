extern crate uu_expand;

fn main() {
    std::process::exit(uu_expand::uumain(std::env::args().collect()));
}
