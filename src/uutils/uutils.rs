#![crate_name = "uutils"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

include!(concat!(env!("OUT_DIR"), "/uutils_crates.rs"));

use std::collections::hash_map::HashMap;
use std::path::Path;
use std::env;

static NAME: &'static str = "uutils";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

include!(concat!(env!("OUT_DIR"), "/uutils_map.rs"));

fn usage(cmap: &UtilityMap) {
    println!("{} {}", NAME, VERSION);
    println!("");
    println!("Usage:");
    println!("  {} [util [arguments...]]\n", NAME);
    println!("Currently defined functions:");
    let mut utils: Vec<&str> = cmap.keys().map(|&s| s).collect();
    utils.sort();
    for util in utils {
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

    if let Some(&uumain) = umap.get(binary_as_util) {
        std::process::exit(uumain(args));
    }

    if binary_as_util.ends_with("uutils") || binary_as_util.starts_with("uutils") ||
     binary_as_util.ends_with("busybox") || binary_as_util.starts_with("busybox") {
        args.remove(0);
    } else {
        let mut found = false;
        for util in umap.keys() {
            if binary_as_util.ends_with(util) {
                args[0] = (*util).to_owned();
                found = true;
                break;
            }
        }
        if ! found {
            println!("{}: applet not found", binary_as_util);
            std::process::exit(1);
        }
    }

    // try first arg as util name.
    if args.len() >= 1 {

        let util = &args[0][..];

        match umap.get(util) {
            Some(&uumain) => {
                std::process::exit(uumain(args.clone()));
            }
            None => {
                if &args[0][..] == "--help" {
                    // see if they want help on a specific util
                    if args.len() >= 2 {
                        let util = &args[1][..];
                        match umap.get(util) {
                            Some(&uumain) => {
                                std::process::exit(uumain(vec![util.to_owned(), "--help".to_owned()]));
                            }
                            None => {
                                println!("{}: applet not found", util);
                                std::process::exit(1);
                            }
                        }
                    }
                    usage(&umap);
                    std::process::exit(0);
                } else {
                    println!("{}: applet not found", util);
                    std::process::exit(1);
                }
            }
        }
    } else {
        // no arguments provided
        usage(&umap);
        std::process::exit(0);
    }
}
