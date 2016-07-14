extern crate uu_install;

fn main() {
    std::process::exit(uu_install::uumain(std::env::args().collect()));
}
