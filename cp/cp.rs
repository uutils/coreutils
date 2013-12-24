#[link(name="cp", vers="1.0.0", author="Jordy Dickinson")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordy Dickinson <jordy.dickinson@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern mod extra;

use std::os;
use std::io::fs;

use conf::Conf;

mod conf;
mod util;

fn main() {
    let conf = Conf::new(os::args());

    match conf.mode {
        conf::Copy    => copy(&conf),
        conf::Help    => help(&conf),
        conf::Version => version(),
    }
}

fn version() {
    println("cp 1.0.0");
}

fn help(conf: &Conf) {
    println("Usage: cp SOURCE DEST");
    println("");
    println(conf.usage);
}

fn copy(conf: &Conf) {
    // We assume there is only one source for now.
    let source = &conf.sources[0];
    let dest = &conf.dest;

    if util::paths_refer_to_same_file(*source, *dest) {
        error!("error: \"{:s}\" and \"{:s}\" are the same file",
               source.display().to_str(),
               dest.display().to_str());
        fail!();
    }

    // In the case of only one source and one destination, it's a simple file
    // copy operation.
    fs::copy(*source, *dest);
}
