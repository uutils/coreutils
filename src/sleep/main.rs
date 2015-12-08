extern crate uu_sleep;

fn main() {
    std::process::exit(uu_sleep::uumain(std::env::args().collect()));
}
