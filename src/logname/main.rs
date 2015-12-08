extern crate uu_logname;

fn main() {
    std::process::exit(uu_logname::uumain(std::env::args().collect()));
}
