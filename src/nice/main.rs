extern crate uu_nice;

fn main() {
    std::process::exit(uu_nice::uumain(std::env::args().collect()));
}
