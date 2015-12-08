extern crate uu_tty;

fn main() {
    std::process::exit(uu_tty::uumain(std::env::args().collect()));
}
