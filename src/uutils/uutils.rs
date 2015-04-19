#![crate_name = "uutils"]
#![feature(exit_status, rustc_private)]

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

use std::env;
use std::collections::hash_map::HashMap;
use std::path::Path;

static NAME: &'static str = "uutils";
static VERSION: &'static str = "1.0.0";

fn util_map() -> HashMap<&'static str, fn(Vec<String>) -> i32> {
    let mut map = HashMap::new();
    @UTIL_MAP@
    map
}

fn usage(cmap: &HashMap<&'static str, fn(Vec<String>) -> i32>) {
    println!("{} {}", NAME, VERSION);
    println!("");
    println!("Usage:");
    println!("  {} [util [arguments...]]\n", NAME);
    println!("Currently defined functions:");
    let mut utils: Vec<&str> = cmap.keys().map(|&s| s).collect();
    utils.sort();
    for util in utils.iter() {
        println!("\t{}", util);
    }
}

fn main() {
    let umap = util_map();
    let mut args : Vec<String> = env::args().collect();

    // try binary name as util name.
    let args0 = args[0].clone();
    let binary = Path::new(&args0[..]);
    let binary_as_util = binary.file_name().unwrap().to_str().unwrap();

    match umap.get(binary_as_util) {
        Some(&uumain) => {
            env::set_exit_status(uumain(args));
            return
        }
        None => (),
    }

    if binary_as_util.ends_with("uutils") || binary_as_util.starts_with("uutils") ||
        binary_as_util.ends_with("busybox") || binary_as_util.starts_with("busybox") {
            // uutils can be called as either "uutils", "busybox"
            // "uutils-suffix" or "busybox-suffix". Not sure
            // what busybox uses the -suffix pattern for.
    } else {
        println!("{}: applet not found", binary_as_util);
        env::set_exit_status(1);
        return
    }

    // try first arg as util name.
    if args.len() >= 2 {
        args.remove(0);
        let util = &args[0][..];

        match umap.get(util) {
            Some(&uumain) => {
                env::set_exit_status(uumain(args.clone()));
                return
            }
            None => {
                if &args[0][..] == "--help" {
                    // see if they want help on a specific util
                    if args.len() >= 2 {
                        let util = &args[1][..];
                        match umap.get(util) {
                            Some(&uumain) => {
                                env::set_exit_status(uumain(vec![util.to_string(), "--help".to_string()]));
                                return
                            }
                            None => {
                                println!("{}: applet not found", util);
                                env::set_exit_status(1);
                                return
                            }
                        }
                    }
                    usage(&umap);
                    env::set_exit_status(0);
                    return
                } else {
                    println!("{}: applet not found", util);
                    env::set_exit_status(1);
                    return
                }
            }
        }
    } else {
        // no arguments provided
        usage(&umap);
        env::set_exit_status(0);
        return
    }
}
