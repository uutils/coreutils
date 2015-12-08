extern crate uu_uptime;

fn main() {
    std::process::exit(uu_uptime::uumain(std::env::args().collect()));
}
