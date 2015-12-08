extern crate uu_pwd;

fn main() {
    std::process::exit(uu_pwd::uumain(std::env::args().collect()));
}
