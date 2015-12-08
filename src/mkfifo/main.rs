extern crate uu_mkfifo;

fn main() {
    std::process::exit(uu_mkfifo::uumain(std::env::args().collect()));
}
