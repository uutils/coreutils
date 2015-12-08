extern crate uu_hostid;

fn main() {
    std::process::exit(uu_hostid::uumain(std::env::args().collect()));
}
