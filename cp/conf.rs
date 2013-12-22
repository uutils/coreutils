/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordy Dickinson <jordy.dickinson@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

use extra::getopts::groups::{
    getopts,
    optflag,
    usage,
};

pub struct Conf {
    progname: ~str,
    usage: ~str,
    mode: Mode,
}

impl Conf {
    pub fn new(args: &[~str]) -> Conf {
        let opts = ~[
            optflag("h", "help", "display this help and exit"),
            optflag("", "version", "output version information and exit"),
            ];
        let matches = match getopts(args.tail(), opts) {
            Ok(m) => m,
            Err(e) => {
                error!("error: {:s}", e.to_err_msg());
                fail!()
            },
        };

        Conf {
            progname: args[0].clone(),
            usage: usage("Copy SOURCE to DEST, or multiple SOURCE(s) to \
                         DIRECTORY.", opts),
            mode: if matches.opt_present("version") {
                Version
            } else if matches.opt_present("help") {
                Help
            } else {
                Copy
            },
        }
    }
}

pub enum Mode {
    Copy,
    Help,
    Version,
}
