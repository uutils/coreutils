extern crate uu_chroot;

fn main() {
    std::process::exit(uu_chroot::uumain(std::env::args().collect()));
}
