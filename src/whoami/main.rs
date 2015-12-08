extern crate uu_whoami;

fn main() {
    std::process::exit(uu_whoami::uumain(std::env::args().collect()));
}
