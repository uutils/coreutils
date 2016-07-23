// TODO: use some sort of byte iterator, instead of passing bytes in u64
pub fn print_item_oct(p: u64, itembytes: usize) {
    let itemwidth = 3 * itembytes;
    let itemspace = 4 * itembytes - itemwidth;

    print!("{:>itemspace$}{:0width$o}",
           "",
           p,
           width = itemwidth,
           itemspace = itemspace);
}

pub fn print_item_hex(p: u64, itembytes: usize) {
    let itemwidth = 2 * itembytes;
    let itemspace = 4 * itembytes - itemwidth;

    print!("{:>itemspace$}{:0width$x}",
           "",
           p,
           width = itemwidth,
           itemspace = itemspace);
}


fn sign_extend(item: u64, itembytes: usize) -> i64{
    let shift = 64 - itembytes * 8;
    (item << shift) as i64 >> shift
}


pub fn print_item_dec_s(p: u64, itembytes: usize) {
    // sign extend
    let s = sign_extend(p,itembytes);
    print!("{:totalwidth$}", s, totalwidth = 4 * itembytes);
}

pub fn print_item_dec_u(p: u64, itembytes: usize) {
    print!("{:totalwidth$}", p, totalwidth = 4 * itembytes);
}
