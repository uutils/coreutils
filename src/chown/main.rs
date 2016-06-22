extern crate uu_chown;

fn main() {
    std::process::exit(uu_chown::uumain(std::env::args().collect()));
}
