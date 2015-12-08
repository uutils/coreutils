extern crate uu_hostname;

fn main() {
    std::process::exit(uu_hostname::uumain(std::env::args().collect()));
}
