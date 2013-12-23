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
use std::io;
use std::io::fs;

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
    // We assume there is only one source for now.
    let source = &conf.sources[0];
    let dest = &conf.dest;
    let mut raw_source = source.clone();
    let mut raw_dest = dest.clone();

    // We need to ensure they're not the same file, so we have to take symlinks
    // and relative paths into account.
    if fs::lstat(raw_source).kind == io::TypeSymlink {
        raw_source = ~fs::readlink(raw_source).unwrap();
    }
    raw_source = ~os::make_absolute(raw_source);
    if fs::lstat(raw_dest).kind == io::TypeSymlink {
        raw_dest = ~fs::readlink(raw_dest).unwrap();
    }
    raw_dest = ~os::make_absolute(raw_dest);

    if raw_source == raw_dest {
        error!("error: \"{:s}\" and \"{:s}\" are the same file",
               source.display().to_str(),
               dest.display().to_str());
        fail!();
    }

    // In the case of only one source and one destination, it's a simple file
    // copy operation.
    fs::copy(*source, *dest);
}
