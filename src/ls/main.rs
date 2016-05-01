extern crate uu_ls;

fn main() {
    std::process::exit(uu_ls::uumain(std::env::args().collect()));
}
