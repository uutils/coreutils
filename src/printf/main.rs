extern crate uu_printf;

fn main() {
    std::process::exit(uu_printf::uumain(std::env::args().collect()));
}
