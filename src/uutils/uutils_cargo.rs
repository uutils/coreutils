#![crate_name = "uutils"]
#![feature(macro_rules)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;

#[cfg(feature="base64")]    extern crate base64;
#[cfg(feature="basename")]  extern crate basename;
#[cfg(feature="cat")]       extern crate cat;
#[cfg(feature="chroot")]    extern crate chroot;
#[cfg(feature="cksum")]     extern crate cksum;
#[cfg(feature="comm")]      extern crate comm;
#[cfg(feature="cp")]        extern crate cp;
#[cfg(feature="cut")]       extern crate cut;
#[cfg(feature="dirname")]   extern crate dirname;
#[cfg(feature="du")]        extern crate du;
#[cfg(feature="echo")]      extern crate echo;
#[cfg(feature="env")]       extern crate env;
#[cfg(feature="expand")]    extern crate expand;
#[cfg(feature="factor")]    extern crate factor;
#[cfg(feature="false")]     extern crate uufalse;
#[cfg(feature="fmt")]       extern crate fmt;
#[cfg(feature="fold")]      extern crate fold;
#[cfg(feature="groups")]    extern crate groups;
#[cfg(feature="hashsum")]   extern crate hashsum;
#[cfg(feature="head")]      extern crate head;
#[cfg(feature="hostid")]    extern crate hostid;
#[cfg(feature="hostname")]  extern crate hostname;
#[cfg(feature="id")]        extern crate id;
#[cfg(feature="kill")]      extern crate kill;
#[cfg(feature="link")]      extern crate link;
#[cfg(feature="logname")]   extern crate logname;
#[cfg(feature="mkdir")]     extern crate mkdir;
#[cfg(feature="mkfifo")]    extern crate mkfifo;
#[cfg(feature="nl")]        extern crate nl;
#[cfg(feature="nohup")]     extern crate nohup;
#[cfg(feature="paste")]     extern crate paste;
#[cfg(feature="printenv")]  extern crate printenv;
#[cfg(feature="pwd")]       extern crate pwd;
#[cfg(feature="realpath")]  extern crate realpath;
#[cfg(feature="relpath")]   extern crate relpath;
#[cfg(feature="rm")]        extern crate rm;
#[cfg(feature="rmdir")]     extern crate rmdir;
#[cfg(feature="seq")]       extern crate seq;
#[cfg(feature="shuf")]      extern crate shuf;
#[cfg(feature="sleep")]     extern crate sleep;
#[cfg(feature="split")]     extern crate split;
#[cfg(feature="sum")]       extern crate sum;
#[cfg(feature="sync")]      extern crate uusync;
#[cfg(feature="tac")]       extern crate tac;
#[cfg(feature="tail")]      extern crate tail;
#[cfg(feature="tee")]       extern crate tee;
#[cfg(feature="test")]      extern crate uutest;
#[cfg(feature="timeout")]   extern crate timeout;
#[cfg(feature="touch")]     extern crate touch;
#[cfg(feature="tr")]        extern crate tr;
#[cfg(feature="true")]      extern crate uutrue;
#[cfg(feature="truncate")]  extern crate truncate;
#[cfg(feature="tsort")]     extern crate tsort;
#[cfg(feature="tty")]       extern crate tty;
#[cfg(feature="uname")]     extern crate uname;
#[cfg(feature="unexpand")]  extern crate unexpand;
#[cfg(feature="uniq")]      extern crate uniq;
#[cfg(feature="unlink")]    extern crate unlink;
#[cfg(feature="uptime")]    extern crate uptime;
#[cfg(feature="users")]     extern crate users;
#[cfg(feature="wc")]        extern crate wc;
#[cfg(feature="whoami")]    extern crate whoami;
#[cfg(feature="yes")]       extern crate yes;

use std::os;
use std::collections::hashmap::HashMap;

static NAME: &'static str = "uutils";
static VERSION: &'static str = "1.0.0";

fn util_map() -> HashMap<&'static str, fn(Vec<String>) -> int> {
    let mut map = HashMap::new();

    match true { #[cfg(feature="base64")]   _ => { map.insert("base64", base64::uumain); } #[cfg(not(feature="base64"))] _ => () };
    match true { #[cfg(feature="basename")] _ => { map.insert("basename", basename::uumain); } #[cfg(not(feature="basename"))] _ => () };
    match true { #[cfg(feature="cat")]	    _ => { map.insert("cat", cat::uumain); } #[cfg(not(feature="cat"))] _ => () };
    match true { #[cfg(feature="chroot")]	_ => { map.insert("chroot", chroot::uumain); } #[cfg(not(feature="chroot"))] _ => () };
    match true { #[cfg(feature="cksum")]	_ => { map.insert("cksum", cksum::uumain); } #[cfg(not(feature="cksum"))] _ => () };
    match true { #[cfg(feature="comm")]	    _ => { map.insert("comm", comm::uumain); } #[cfg(not(feature="comm"))] _ => () };
    match true { #[cfg(feature="cp")]	    _ => { map.insert("cp", cp::uumain); } #[cfg(not(feature="cp"))] _ => () };
    match true { #[cfg(feature="cut")]	    _ => { map.insert("cut", cut::uumain); } #[cfg(not(feature="cut"))] _ => () };
    match true { #[cfg(feature="dirname")]	_ => { map.insert("dirname", dirname::uumain); } #[cfg(not(feature="dirname"))] _ => () };
    match true { #[cfg(feature="du")]	    _ => { map.insert("du", du::uumain); } #[cfg(not(feature="du"))] _ => () };
    match true { #[cfg(feature="echo")]	    _ => { map.insert("echo", echo::uumain); } #[cfg(not(feature="echo"))] _ => () };
    match true { #[cfg(feature="env")]	    _ => { map.insert("env", env::uumain); } #[cfg(not(feature="env"))] _ => () };
    match true { #[cfg(feature="expand")]	_ => { map.insert("expand", expand::uumain); } #[cfg(not(feature="expand"))] _ => () };
    match true { #[cfg(feature="factor")]	_ => { map.insert("factor", factor::uumain); } #[cfg(not(feature="factor"))] _ => () };
    match true { #[cfg(feature="false")]	_ => { map.insert("false", uufalse::uumain); } #[cfg(not(feature="false"))] _ => () };
    match true { #[cfg(feature="fmt")]	    _ => { map.insert("fmt", fmt::uumain); } #[cfg(not(feature="fmt"))] _ => () };
    match true { #[cfg(feature="fold")]	    _ => { map.insert("fold", fold::uumain); } #[cfg(not(feature="fold"))] _ => () };
    match true { #[cfg(feature="groups")]	_ => { map.insert("groups", groups::uumain); } #[cfg(not(feature="groups"))] _ => () };
    match true { #[cfg(feature="hashsum")]	_ => {
        map.insert("hashsum", hashsum::uumain);
        map.insert("md5sum", hashsum::uumain);
        map.insert("sha1sum", hashsum::uumain);
        map.insert("sha224sum", hashsum::uumain);
        map.insert("sha256sum", hashsum::uumain);
        map.insert("sha384sum", hashsum::uumain);
        map.insert("sha512sum", hashsum::uumain);
    } #[cfg(not(feature="hashsum"))] _ => ()};
    match true { #[cfg(feature="head")]	    _ => { map.insert("head", head::uumain); } #[cfg(not(feature="head"))] _ => () };
    match true { #[cfg(feature="hostid")]	_ => { map.insert("hostid", hostid::uumain); } #[cfg(not(feature="hostid"))] _ => () };
    match true { #[cfg(feature="hostname")]	_ => { map.insert("hostname", hostname::uumain); } #[cfg(not(feature="hostname"))] _ => () };
    match true { #[cfg(feature="id")]	    _ => { map.insert("id", id::uumain); } #[cfg(not(feature="id"))] _ => () };
    match true { #[cfg(feature="kill")]	    _ => { map.insert("kill", kill::uumain); } #[cfg(not(feature="kill"))] _ => () };
    match true { #[cfg(feature="link")]	    _ => { map.insert("link", link::uumain); } #[cfg(not(feature="link"))] _ => () };
    match true { #[cfg(feature="logname")]	_ => { map.insert("logname", logname::uumain); } #[cfg(not(feature="logname"))] _ => () };
    match true { #[cfg(feature="mkdir")]	_ => { map.insert("mkdir", mkdir::uumain); } #[cfg(not(feature="mkdir"))] _ => () };
    match true { #[cfg(feature="mkfifo")]	_ => { map.insert("mkfifo", mkfifo::uumain); } #[cfg(not(feature="mkfifo"))] _ => () };
    match true { #[cfg(feature="nl")]	    _ => { map.insert("nl", nl::uumain); } #[cfg(not(feature="nl"))] _ => () };
    match true { #[cfg(feature="nohup")]	_ => { map.insert("nohup", nohup::uumain); } #[cfg(not(feature="nohup"))] _ => () };
    match true { #[cfg(feature="paste")]	_ => { map.insert("paste", paste::uumain); } #[cfg(not(feature="paste"))] _ => () };
    match true { #[cfg(feature="printenv")]	_ => { map.insert("printenv", printenv::uumain); } #[cfg(not(feature="printenv"))] _ => () };
    match true { #[cfg(feature="pwd")]	    _ => { map.insert("pwd", pwd::uumain); } #[cfg(not(feature="pwd"))] _ => () };
    match true { #[cfg(feature="realpath")]	_ => { map.insert("realpath", realpath::uumain); } #[cfg(not(feature="realpath"))] _ => () };
    match true { #[cfg(feature="relpath")]	_ => { map.insert("relpath", relpath::uumain); } #[cfg(not(feature="relpath"))] _ => () };
    match true { #[cfg(feature="rm")]	    _ => { map.insert("rm", rm::uumain); } #[cfg(not(feature="rm"))] _ => () };
    match true { #[cfg(feature="rmdir")]	_ => { map.insert("rmdir", rmdir::uumain); } #[cfg(not(feature="rmdir"))] _ => () };
    match true { #[cfg(feature="seq")]	    _ => { map.insert("seq", seq::uumain); } #[cfg(not(feature="seq"))] _ => () };
    match true { #[cfg(feature="shuf")]	    _ => { map.insert("shuf", shuf::uumain); } #[cfg(not(feature="shuf"))] _ => () };
    match true { #[cfg(feature="sleep")]	_ => { map.insert("sleep", sleep::uumain); } #[cfg(not(feature="sleep"))] _ => () };
    match true { #[cfg(feature="split")]	_ => { map.insert("split", split::uumain); } #[cfg(not(feature="split"))] _ => () };
    match true { #[cfg(feature="sum")]	    _ => { map.insert("sum", sum::uumain); } #[cfg(not(feature="sum"))] _ => () };
    match true { #[cfg(feature="sync")]	    _ => { map.insert("sync", uusync::uumain); } #[cfg(not(feature="sync"))] _ => () };
    match true { #[cfg(feature="tac")]	    _ => { map.insert("tac", tac::uumain); } #[cfg(not(feature="tac"))] _ => () };
    match true { #[cfg(feature="tail")]	    _ => { map.insert("tail", tail::uumain); } #[cfg(not(feature="tail"))] _ => () };
    match true { #[cfg(feature="tee")]	    _ => { map.insert("tee", tee::uumain); } #[cfg(not(feature="tee"))] _ => () };
    match true { #[cfg(feature="test")]	    _ => { map.insert("test", uutest::uumain); } #[cfg(not(feature="test"))] _ => () };
    match true { #[cfg(feature="timeout")]	_ => { map.insert("timeout", timeout::uumain); } #[cfg(not(feature="timeout"))] _ => () };
    match true { #[cfg(feature="touch")]	_ => { map.insert("touch", touch::uumain); } #[cfg(not(feature="touch"))] _ => () };
    match true { #[cfg(feature="tr")]	    _ => { map.insert("tr", tr::uumain); } #[cfg(not(feature="tr"))] _ => () };
    match true { #[cfg(feature="true")]	    _ => { map.insert("true", uutrue::uumain); } #[cfg(not(feature="true"))] _ => () };
    match true { #[cfg(feature="truncate")]	_ => { map.insert("truncate", truncate::uumain); } #[cfg(not(feature="truncate"))] _ => () };
    match true { #[cfg(feature="tsort")]	_ => { map.insert("tsort", tsort::uumain); } #[cfg(not(feature="tsort"))] _ => () };
    match true { #[cfg(feature="tty")]	    _ => { map.insert("tty", tty::uumain); } #[cfg(not(feature="tty"))] _ => () };
    match true { #[cfg(feature="uname")]	_ => { map.insert("uname", uname::uumain); } #[cfg(not(feature="uname"))] _ => () };
    match true { #[cfg(feature="unexpand")]	_ => { map.insert("unexpand", unexpand::uumain); } #[cfg(not(feature="unexpand"))] _ => () };
    match true { #[cfg(feature="uniq")]	    _ => { map.insert("uniq", uniq::uumain); } #[cfg(not(feature="uniq"))] _ => () };
    match true { #[cfg(feature="unlink")]	_ => { map.insert("unlink", unlink::uumain); } #[cfg(not(feature="unlink"))] _ => () };
    match true { #[cfg(feature="uptime")]	_ => { map.insert("uptime", uptime::uumain); } #[cfg(not(feature="uptime"))] _ => () };
    match true { #[cfg(feature="users")]	_ => { map.insert("users", users::uumain); } #[cfg(not(feature="users"))] _ => () };
    match true { #[cfg(feature="wc")]	    _ => { map.insert("wc", wc::uumain); } #[cfg(not(feature="wc"))] _ => () };
    match true { #[cfg(feature="whoami")]	_ => { map.insert("whoami", whoami::uumain); } #[cfg(not(feature="whoami"))] _ => () };
    match true { #[cfg(feature="yes")]	    _ => { map.insert("yes", yes::uumain); } #[cfg(not(feature="yes"))] _ => () };

    map
}

fn usage(cmap: &HashMap<&'static str, fn(Vec<String>) -> int>) {
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
    let mut args = os::args();

    // try binary name as util name.
    let binary = Path::new(args[0].as_slice());
    let binary_as_util = binary.filename_str().unwrap();

    match umap.find_equiv(&binary_as_util) {
        Some(&uumain) => {
            os::set_exit_status(uumain(args));
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
        os::set_exit_status(1);
        return
    }

    // try first arg as util name.
    if args.len() >= 2 {
        args.remove(0);
        let util = args[0].as_slice();

        match umap.find_equiv(&util) {
            Some(&uumain) => {
                os::set_exit_status(uumain(args.clone()));
                return
            }
            None => {
                if args[0].as_slice() == "--help" {
                    // see if they want help on a specific util
                    if args.len() >= 2 {
                        let util = args[1].as_slice();
                        match umap.find_equiv(&util) {
                            Some(&uumain) => {
                                os::set_exit_status(uumain(vec![util.to_string(), "--help".to_string()]));
                                return
                            }
                            None => {
                                println!("{}: applet not found", util);
                                os::set_exit_status(1);
                                return
                            }
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
            }
        }
    } else {
        // no arguments provided
        usage(&umap);
        os::set_exit_status(0);
        return
    }
}
