#![crate_id(name="uutils", vers="1.0.0", author="Michael Gehring")]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

@CRATES@

use std::os;
use std::collections::hashmap::HashMap;

static NAME: &'static str = "uutils";
static VERSION: &'static str = "1.0.0";

fn util_map() -> HashMap<&str, fn(Vec<String>) -> int> {
    fn uutrue(_: Vec<String>) -> int { 0 }
    fn uufalse(_: Vec<String>) -> int { 1 }

    let mut map = HashMap::<&str, fn(Vec<String>) -> int>::new();
    @UTIL_MAP@
    map
}

fn usage(cmap: &HashMap<&str, fn(Vec<String>) -> int>) {
    println!("{} {}", NAME, VERSION);
    println!("");
    println!("Usage:");
    println!("  {} [util [arguments...]", NAME);
    println!("Currently defined functions:");
    let mut utils: Vec<&str> = cmap.keys().map(|&s| s).collect();
    utils.sort();
    for util in utils.iter() {
        println!("\t{}", util);
    }
    println!("");
}

fn main() {
    let umap = util_map();
    let mut args = os::args();

    // try binary name as util name.
    let binary = Path::new(args.get(0).as_slice());
    let binary_as_util = binary.filename_str().unwrap();
    if umap.contains_key(&binary_as_util) {
        let &uumain = umap.get(&binary_as_util);
        os::set_exit_status(uumain(args));
        return
    } else if binary_as_util.starts_with("uutils")
        || binary_as_util.starts_with("busybox") {
            // uutils can be called as either "uutils", "busybox"
            // "uutils-suffix" or "busybox-suffix". Not sure
            // what busybox uses the -suffix pattern for.
    } else {
        println!("{}: applet not found", binary_as_util);
        os::set_exit_status(1);
        return
    }

    // try first arg as util name.
    if args.len() >= 2 {
        args.shift();
        let util = args.get(0).as_slice();
        if umap.contains_key(&util) {
            let &uumain = umap.get(&util);
            os::set_exit_status(uumain(args.clone()));
            return
        } else if args.get(0).as_slice() == "--help" {
            // see if they want help on a specific util
            if args.len() >= 2 {
                let util = args.get(1).as_slice();
                if umap.contains_key(&util) {
                    let &uumain = umap.get(&util);
                    os::set_exit_status(uumain(vec!["--help".to_string()]));
                    return
                } else {
                    println!("{}: applet not found", util);
                    os::set_exit_status(1);
                    return
                }
            }
            usage(&umap);
            os::set_exit_status(0);
            return
        } else {
            println!("{}: applet not found", util);
            os::set_exit_status(1);
            return
        }
    } else {
        // no arguments provided
        usage(&umap);
        os::set_exit_status(0);
        return
    }
}
