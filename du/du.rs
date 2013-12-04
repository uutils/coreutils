#[link(name="du", vers="1.0.0", author="Derek Chiang")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern mod extra;

use std::os;
use std::io::stderr;
use std::io::fs;
use std::io::FileStat;
use std::path::Path;
use std::uint;
use std::comm::{Port, Chan, SharedChan, stream};
use extra::arc::Arc;
use extra::future::Future;
use extra::getopts::groups;

static VERSION: &'static str = "1.0.0";

fn du(path: &Path) -> ~[Arc<FileStat>] {
    let mut stats = ~[];
    let mut futures = ~[];
    stats.push(Arc::new(path.stat()));

    for f in fs::readdir(path).move_iter() {
        match f.is_file() {
            true => stats.push(Arc::new(f.stat())),
            false => futures.push(do Future::spawn { du(&f) })
        }
    }

    for future in futures.mut_iter() {
        stats.push_all(future.get());
    }

    return stats;
}

fn main() {
    let args = os::args();
    let program = args[0].clone();
    let opts = ~[
        groups::optflag("a", "all", " write counts for all files, not just directories"),
        groups::optflag("", "apparent-size", "print apparent sizes,  rather  than  disk  usage;
            although  the apparent  size is usually smaller, it may be larger due to holes
            in ('sparse') files, internal  fragmentation,  indirect  blocks, and the like"),
        groups::optopt("B", "block-size", "scale sizes  by  SIZE before printing them.
            E.g., '-BM' prints sizes in units of 1,048,576 bytes.  See SIZE format below.",
            "SIZE"),
        groups::optflag("", "help", "display this help and exit"),
        groups::optflag("", "version", "output version information and exit"),
    ];

    let matches = match groups::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            writeln!(&mut stderr() as &mut Writer,
                "Invalid options\n{}", f.to_err_msg());
            os::set_exit_status(1);
            return
        }
    };

    if matches.opt_present("help") {
        println("du " + VERSION + " - estimate file space usage");
        println("");
        println("Usage:");
        println!("  {0:s} [OPTION]... [FILE]...", program);
        println!("  {0:s} [OPTION]... --files0-from=F", program);
        println("");
        println(groups::usage("Summarize disk usage of each FILE, recursively for directories.", opts));
        return;
    }

    if !matches.free.is_empty() {
    }

    let strs = match matches.free {
        [] => ~[~"./"],
        x => x
    };

    for path_str in strs.iter() {
        let path = Path::init(path_str.clone());
        for stat_arc in du(&path).move_iter() {
            let stat = stat_arc.get();
            println!("{:<10} {}", stat.size, stat.path.display());
        }
    }
}
