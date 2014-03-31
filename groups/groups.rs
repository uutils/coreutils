#![crate_id(name="groups", vers="1.0.0", author="Alan Andrade")]
/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alan Andrade <alan.andradec@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 *
 */
#![feature(macro_rules)]

extern crate getopts;

use std::os;
use getopts::{
    optflag,
    getopts,
    usage
};
use c_types::{get_pw_from_args, group};

#[path = "../common/util.rs"] mod util;
#[path = "../common/c_types.rs"] mod c_types;

static NAME: &'static str = "groups";

fn main () {
    let args = os::args();
    let options = [
            optflag("h", "", "Help")
        ];

    let matches = match getopts(args.tail(), options) {
        Ok(m) => { m },
        Err(_) => {
            show_error!(1, "{}", usage(NAME, options));
            return;
        }
    };

    group(get_pw_from_args(&matches.free), true);
}
