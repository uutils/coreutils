extern crate uu_printenv;

fn main() {
    std::process::exit(uu_printenv::uumain(std::env::args().collect()));
}
