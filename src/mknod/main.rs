extern crate uu_mknod;

fn main() {
    std::process::exit(uu_mknod::uumain(std::env::args().collect()));
}
