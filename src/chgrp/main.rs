extern crate uu_chgrp;

fn main() {
    std::process::exit(uu_chgrp::uumain(std::env::args().collect()));
}
