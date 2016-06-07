extern crate uu_stat;

fn main() {
    std::process::exit(uu_stat::uumain(std::env::args().collect()));
}
