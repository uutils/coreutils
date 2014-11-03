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
#[cfg(feature="mv")]        extern crate mv;
#[cfg(feature="nl")]        extern crate nl;
#[cfg(feature="nproc")]     extern crate nproc;
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
#[cfg(feature="sort")]      extern crate sort;
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
use std::collections::hash_map::HashMap;

static NAME: &'static str = "uutils";
static VERSION: &'static str = "1.0.0";

fn util_map() -> HashMap<&'static str, fn(Vec<String>) -> int> {
    let mut map = HashMap::new();

    macro_rules! add_util(
        ($guard:meta, $util:expr, $crte:ident) => ( 
            match true {
                #[cfg($guard)]
                _ => { map.insert($util, $crte::uumain);} 
                #[cfg(not($guard))]
                _ => ()
            }
        )
    )

    add_util!(feature="base64", "base64", base64);
    add_util!(feature="basename", "basename", basename);
    add_util!(feature="cat", "cat", cat);
    add_util!(feature="chroot", "chroot", chroot);
    add_util!(feature="cksum", "cksum", cksum);
    add_util!(feature="comm", "comm", comm);
    add_util!(feature="cp", "cp", cp);
    add_util!(feature="cut", "cut", cut);
    add_util!(feature="dirname", "dirname", dirname);
    add_util!(feature="du", "du", du);
    add_util!(feature="echo", "echo", echo);
    add_util!(feature="env", "env", env);
    add_util!(feature="expand", "expand", expand);
    add_util!(feature="factor", "factor", factor);
    add_util!(feature="false", "false", uufalse);
    add_util!(feature="fmt", "fmt", fmt);
    add_util!(feature="fold", "fold", fold);
    add_util!(feature="groups", "groups", groups);
    add_util!(feature="hashsum", "hashsum", hashsum);
    add_util!(feature="hashsum", "md5sum", hashsum);
    add_util!(feature="hashsum", "sha1sum", hashsum);
    add_util!(feature="hashsum", "sha224sum", hashsum);
    add_util!(feature="hashsum", "sha256sum", hashsum);
    add_util!(feature="hashsum", "sha384sum", hashsum);
    add_util!(feature="hashsum", "sha512sum", hashsum);
    add_util!(feature="head", "head", head);
    add_util!(feature="hostid", "hostid", hostid);
    add_util!(feature="hostname", "hostname", hostname);
    add_util!(feature="id", "id", id);
    add_util!(feature="kill", "kill", kill);
    add_util!(feature="link", "link", link);
    add_util!(feature="logname", "logname", logname);
    add_util!(feature="mkdir", "mkdir", mkdir);
    add_util!(feature="mkfifo", "mkfifo", mkfifo);
    add_util!(feature="mv", "mv", mv);
    add_util!(feature="nl", "nl", nl);
    add_util!(feature="nproc", "nproc", nproc);
    add_util!(feature="nohup", "nohup", nohup);
    add_util!(feature="paste", "paste", paste);
    add_util!(feature="printenv", "printenv", printenv);
    add_util!(feature="pwd", "pwd", pwd);
    add_util!(feature="realpath", "realpath", realpath);
    add_util!(feature="relpath", "relpath", relpath);
    add_util!(feature="rm", "rm", rm);
    add_util!(feature="rmdir", "rmdir", rmdir);
    add_util!(feature="seq", "seq", seq);
    add_util!(feature="shuf", "shuf", shuf);
    add_util!(feature="sleep", "sleep", sleep);
    add_util!(feature="sort", "sort", sort);
    add_util!(feature="split", "split", split);
    add_util!(feature="sum", "sum", sum);
    add_util!(feature="sync", "sync", uusync);
    add_util!(feature="tac", "tac", tac);
    add_util!(feature="tail", "tail", tail);
    add_util!(feature="tee", "tee", tee);
    add_util!(feature="test", "test", uutest);
    add_util!(feature="timeout", "timeout", timeout);
    add_util!(feature="touch", "touch", touch);
    add_util!(feature="tr", "tr", tr);
    add_util!(feature="true", "true", uutrue);
    add_util!(feature="truncate", "truncate", truncate);
    add_util!(feature="tsort", "tsort", tsort);
    add_util!(feature="tty", "tty", tty);
    add_util!(feature="uname", "uname", uname);
    add_util!(feature="unexpand", "unexpand", unexpand);
    add_util!(feature="uniq", "uniq", uniq);
    add_util!(feature="unlink", "unlink", unlink);
    add_util!(feature="uptime", "uptime", uptime);
    add_util!(feature="users", "users", users);
    add_util!(feature="wc", "wc", wc);
    add_util!(feature="whoami", "whoami", whoami);
    add_util!(feature="yes", "yes", yes);

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

    match umap.find_equiv(binary_as_util) {
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

        match umap.find_equiv(util) {
            Some(&uumain) => {
                os::set_exit_status(uumain(args.clone()));
                return
            }
            None => {
                if args[0].as_slice() == "--help" {
                    // see if they want help on a specific util
                    if args.len() >= 2 {
                        let util = args[1].as_slice();
                        match umap.find_equiv(util) {
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
