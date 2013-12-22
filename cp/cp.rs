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

use conf::Conf;

mod conf;

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
}
