extern crate uu_uname;

fn main() {
    std::process::exit(uu_uname::uumain(std::env::args().collect()));
}
