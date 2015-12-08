extern crate uu_tail;

fn main() {
    std::process::exit(uu_tail::uumain(std::env::args().collect()));
}
