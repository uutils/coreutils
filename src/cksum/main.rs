extern crate uu_cksum;

fn main() {
    std::process::exit(uu_cksum::uumain(std::env::args().collect()));
}
