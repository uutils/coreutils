extern crate uu_timeout;

fn main() {
    std::process::exit(uu_timeout::uumain(std::env::args().collect()));
}
