extern crate uu_who;

fn main() {
    std::process::exit(uu_who::uumain(std::env::args().collect()));
}
