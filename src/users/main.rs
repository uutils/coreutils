extern crate uu_users;

fn main() {
    std::process::exit(uu_users::uumain(std::env::args().collect()));
}
