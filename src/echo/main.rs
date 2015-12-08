extern crate uu_echo;

fn main() {
    std::process::exit(uu_echo::uumain(std::env::args().collect()));
}
