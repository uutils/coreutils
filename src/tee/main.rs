extern crate uu_tee;

fn main() {
    std::process::exit(uu_tee::uumain(std::env::args().collect()));
}
