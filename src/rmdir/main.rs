extern crate uu_rmdir;

fn main() {
    std::process::exit(uu_rmdir::uumain(std::env::args().collect()));
}
