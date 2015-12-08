extern crate uu_env;

fn main() {
    std::process::exit(uu_env::uumain(std::env::args().collect()));
}
