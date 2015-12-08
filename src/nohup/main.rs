extern crate uu_nohup;

fn main() {
    std::process::exit(uu_nohup::uumain(std::env::args().collect()));
}
