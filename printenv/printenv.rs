#[link(name="printenv", vers="1.0.0", author="Seldaek")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: printenv (GNU coreutils) 8.13 */

extern mod extra;

use std::os;
use std::io::stderr;
use extra::getopts::*;

fn main() {
    let args = os::args();
    let program = copy args[0];
    let opts = ~[
        groups::optflag("0", "null", "end each output line with 0 byte rather than newline"),
        groups::optflag("h", "help", "display this help and exit"),
        groups::optflag("V", "version", "output version information and exit"),
    ];
    let matches = match groups::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            stderr().write_line("Invalid options");
            stderr().write_line(fail_str(f));
            os::set_exit_status(1);
            return
        }
    };
    if opts_present(&matches, [~"h", ~"help"]) {
        println("printenv 1.0.0");
        println("");
        println("Usage:");
        println(fmt!("  %s [VARIABLE]... [OPTION]...", program));
        println("");
        print(groups::usage("Prints the given environment VARIABLE(s), otherwise prints them all.", opts));
        return;
    }
    if opts_present(&matches, [~"V", ~"version"]) {
        println("printenv 1.0.0");
        return;
    }
    let mut separator = "\n";
    if opts_present(&matches, [~"0", ~"null"]) {
        separator = "\x00";
    };

    exec(matches.free, separator);
}

pub fn exec(args: ~[~str], separator: &str) {
    if args.is_empty() {
        let vars = os::env();
        for vars.iter().advance |&(env_var, value)| {
            print(fmt!("%s=%s", env_var, value));
            print(separator);
        }
        return;
    }

    for args.iter().advance |env_var| {
        match os::getenv(*env_var) {
            Some(var) => {
                print(var);
                print(separator);
            }
            _ => ()
        }
    }
}
