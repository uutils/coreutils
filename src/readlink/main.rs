extern crate uu_readlink;

fn main() {
    std::process::exit(uu_readlink::uumain(std::env::args().collect()));
}
