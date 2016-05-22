extern crate uu_pathchk;

fn main() {
    std::process::exit(uu_pathchk::uumain(std::env::args().collect()));
}
