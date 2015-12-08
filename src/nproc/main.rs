extern crate uu_nproc;

fn main() {
    std::process::exit(uu_nproc::uumain(std::env::args().collect()));
}
