extern crate uu_basename;

fn main() {
    std::process::exit(uu_basename::uumain(std::env::args().collect()));
}
