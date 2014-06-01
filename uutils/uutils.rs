#![crate_id(name="uutils", vers="1.0.0", author="Michael Gehring")]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate collections;
extern crate getopts;

extern crate base64;
extern crate basename;
extern crate cat;
extern crate cksum;
extern crate comm;
extern crate cp;
extern crate dirname;
extern crate du;
extern crate echo;
extern crate env;
extern crate fold;
extern crate groups;
extern crate head;
extern crate hostid;
extern crate hostname;
extern crate id;
extern crate kill;
extern crate logname;
extern crate mkdir;
extern crate paste;
extern crate printenv;
extern crate pwd;
extern crate rm;
extern crate rmdir;
extern crate seq;
extern crate sleep;
extern crate sum;
extern crate tac;
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
use collections::hashmap::HashMap;

static NAME: &'static str = "uutils";
static VERSION: &'static str = "1.0.0";

fn util_map() -> HashMap<&str, fn(Vec<String>)> {
    fn uutrue(_: Vec<String>) { os::set_exit_status(0); }
    fn uufalse(_: Vec<String>) { os::set_exit_status(1); }

    let mut map = HashMap::new();
    map.insert("base64", base64::uumain);
    map.insert("basename", basename::uumain);
    map.insert("cat", cat::uumain);
    map.insert("cksum", cksum::uumain);
    map.insert("comm", comm::uumain);
    map.insert("cp", cp::uumain);
    map.insert("dirname", dirname::uumain);
    map.insert("du", du::uumain);
    map.insert("echo", echo::uumain);
    map.insert("env", env::uumain);
    map.insert("false", uufalse);
    map.insert("fold", fold::uumain);
    map.insert("groups", groups::uumain);
    map.insert("head", head::uumain);
    map.insert("hostid", hostid::uumain);
    map.insert("hostname", hostname::uumain);
    map.insert("id", id::uumain);
    map.insert("kill", kill::uumain);
    map.insert("logname", logname::uumain);
    map.insert("mkdir", mkdir::uumain);
    map.insert("paste", paste::uumain);
    map.insert("printenv", printenv::uumain);
    map.insert("pwd", pwd::uumain);
    map.insert("rm", rm::uumain);
    map.insert("rmdir", rmdir::uumain);
    map.insert("seq", seq::uumain);
    map.insert("sleep", sleep::uumain);
    map.insert("sum", sum::uumain);
    map.insert("tac", tac::uumain);
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

fn usage(cmap: &HashMap<&str, fn(Vec<String>)>) {
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
}

fn main() {
    let umap = util_map();
    let mut args = os::args();

    // try binary name as util name.
    let binary = Path::new(args.get(0).as_slice());
    let util = binary.filename_str().unwrap();
    if umap.contains_key(&util) {
        let &uumain = umap.get(&util);
        uumain(args);
        return
    }

    // try first arg as util name.
    if args.len() >= 2 {
        args.shift();
        let util = args.get(0).as_slice().clone();
        if umap.contains_key(&util) {
            let &uumain = umap.get(&util);
            uumain(args.clone());
            return
        }
    }

    usage(&umap);
    os::set_exit_status(1);
}
