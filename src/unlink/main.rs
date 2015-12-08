extern crate uu_unlink;

fn main() {
    std::process::exit(uu_unlink::uumain(std::env::args().collect()));
}
