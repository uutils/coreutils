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

extern crate base64;
extern crate basename;
extern crate cat;
extern crate chroot;
extern crate cksum;
extern crate comm;
extern crate cp;
extern crate dirname;
extern crate du;
extern crate echo;
extern crate env;
extern crate factor;
extern crate fmt;
extern crate fold;
extern crate groups;
extern crate head;
extern crate hostid;
extern crate hostname;
extern crate id;
extern crate kill;
extern crate logname;
extern crate mkdir;
extern crate nl;
extern crate paste;
extern crate printenv;
extern crate pwd;
extern crate rm;
extern crate rmdir;
extern crate seq;
extern crate sleep;
extern crate sum;
extern crate uusync;
extern crate tac;
extern crate tail;
extern crate tee;
extern crate touch;
extern crate tr;
extern crate truncate;
extern crate tty;
extern crate uname;
extern crate unlink;
extern crate uptime;
extern crate users;
extern crate wc;
extern crate whoami;
extern crate yes;

use std::os;
use std::collections::hashmap::HashMap;

static NAME: &'static str = "uutils";
static VERSION: &'static str = "1.0.0";

fn util_map() -> HashMap<&str, fn(Vec<String>) -> int> {
    fn uutrue(_: Vec<String>) -> int { 0 }
    fn uufalse(_: Vec<String>) -> int { 1 }

    let mut map = HashMap::new();
    map.insert("base64", base64::uumain);
    map.insert("basename", basename::uumain);
    map.insert("cat", cat::uumain);
    map.insert("chroot", chroot::uumain);
    map.insert("cksum", cksum::uumain);
    map.insert("comm", comm::uumain);
    map.insert("cp", cp::uumain);
    map.insert("dirname", dirname::uumain);
    map.insert("du", du::uumain);
    map.insert("echo", echo::uumain);
    map.insert("env", env::uumain);
    map.insert("factor", factor::uumain);
    map.insert("false", uufalse);
    map.insert("fmt", fmt::uumain);
    map.insert("fold", fold::uumain);
    map.insert("groups", groups::uumain);
    map.insert("head", head::uumain);
    map.insert("hostid", hostid::uumain);
    map.insert("hostname", hostname::uumain);
    map.insert("id", id::uumain);
    map.insert("kill", kill::uumain);
    map.insert("logname", logname::uumain);
    map.insert("mkdir", mkdir::uumain);
    map.insert("nl", nl::uumain);
    map.insert("paste", paste::uumain);
    map.insert("printenv", printenv::uumain);
    map.insert("pwd", pwd::uumain);
    map.insert("rm", rm::uumain);
    map.insert("rmdir", rmdir::uumain);
    map.insert("seq", seq::uumain);
    map.insert("sleep", sleep::uumain);
    map.insert("sum", sum::uumain);
    map.insert("sync", uusync::uumain);
    map.insert("tac", tac::uumain);
    map.insert("tail", tail::uumain);
    map.insert("tee", tee::uumain);
    map.insert("touch", touch::uumain);
    map.insert("tr", tr::uumain);
    map.insert("true", uutrue);
    map.insert("truncate", truncate::uumain);
    map.insert("tty", tty::uumain);
    map.insert("uname", uname::uumain);
    map.insert("unlink", unlink::uumain);
    map.insert("uptime", uptime::uumain);
    map.insert("users", users::uumain);
    map.insert("wc", wc::uumain);
    map.insert("whoami", whoami::uumain);
    map.insert("yes", yes::uumain);
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
