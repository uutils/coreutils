#![crate_name = "uu_chgrp"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Maciej Dziardziel <fiedzia@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

//TODO: handle verbosity and dereferencing symbolic links

#![allow(unused_variables)]  // only necessary while the TODOs still exist
#![feature(convert)]

extern crate getopts;
extern crate libc;
extern crate memchr;
extern crate walkdir;

#[macro_use]
extern crate uucore;

use getopts::{Matches, Options};
use std::io::{Error, Write, Result};
use std::mem;
use std::path::Path;
use uucore::c_types::get_group;
use libc::{gid_t, chown};

use walkdir::WalkDir;

const NAME: &'static str = "chgrp";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub enum Verbosity {
    Quiet,
    Default,
    Changes,
    Verbose,
}

impl Verbosity {
    fn from_matches(matches: &Matches) -> Verbosity{
        if matches.opt_present("quiet") || matches.opt_present("silent")
            { Verbosity::Quiet }
        else if matches.opt_present("changes")
            { Verbosity::Changes }
        else if matches.opt_present("verbose")
            { Verbosity::Verbose }
        else
            { Verbosity::Default }
    }
}


pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();
    opts.optflag("c", "changes", "like verbose but report only when a change is made");
    opts.optflag("f", "quiet", "suppress most error messages");
    opts.optflag("", "silent", "suppress most error messages");
    opts.optflag("v", "verbose", "output a diagnostic for every file processed");
    opts.optflag("", "dereference", "affect the referent of each symbolic link (this is the default), rather than the symbolic link itself");
    opts.optflag("h", "no-dereference", "affect symbolic links instead of any referenced file (useful only on systems that can change the ownership of a symlink)");
    opts.optflag("", "no-preserve-root", "do not treat '/' specially (the default)");
    opts.optflag("", "preserve-root", "fail to operate recursively on '/'");
    opts.optflagopt("", "reference", "use RFILE's mode instead of MODE values", "RFILE");
    opts.optflag("R", "recursive", "change files and directories recursively");
    opts.optflag("H", "", "if a command line argument is a symbolic link to a directory, traverse it");
    opts.optflag("L", "", "traverse every symbolic link to a directory encountered");
    opts.optflag("P", "", "do not traverse any symbolic links (default)");
    opts.optflag("", "help", "display this help and exit");
    opts.optflag("", "version", "output version information and exit");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => { crash!(1, "{}", f) }
    };
    if matches.opt_present("help") {
        let msg = format!("{name} {version}

Usage: {program} [OPTION]... GROUP FILE...
  or:  {program} [OPTION]... --reference=RFILE FILE...
Change the group of each FILE to GROU
With --reference, change the group of each FILE to that of RFILE.

  -c, --changes          like verbose but report only when a change is made
  -f, --silent, --quiet  suppress most error messages
  -v, --verbose          output a diagnostic for every file processed
      --dereference      affect the referent of each symbolic link (this is
                         the default), rather than the symbolic link itself
  -h, --no-dereference   affect symbolic links instead of any referenced file
                         (useful only on systems that can change the
                         ownership of a symlink)
      --no-preserve-root  do not treat '/' specially (the default)
      --preserve-root    fail to operate recursively on '/'
      --reference=RFILE  use RFILE's group rather than specifying a
                         GROUP value
  -R, --recursive        operate on files and directories recursively

The following options modify how a hierarchy is traversed when the -R
option is also specified.  If more than one is specified, only the final
one takes effect.

  -H                     if a command line argument is a symbolic link
                         to a directory, traverse it
  -L                     traverse every symbolic link to a directory
                         encountered
  -P                     do not traverse any symbolic links (default)

      --help     display this help and exit
      --version  output version information and exit

Examples:
  {program} staff /u      Change the group of /u to \"staff\".
  {program} -hR staff /u  Change the group of /u and subfiles to \"staff\".",
            name = NAME, version = VERSION, program = NAME);

        print!("{}", opts.usage(&msg));
        return 0;
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    } else if matches.free.is_empty() || (matches.opt_present("reference") && matches.free.is_empty() ) || (!matches.opt_present("reference") && matches.free.len() < 2) {
        show_error!("missing an argument");
        show_error!("for help, try '{} --help'", NAME);
        return 1;
    } 
        
    let changes = matches.opt_present("changes");
    let quiet = matches.opt_present("quiet");
    let verbose = matches.opt_present("verbose");
    let preserve_root = matches.opt_present("preserve-root");
    let free_first_file_index = if matches.opt_str("reference").is_some() {0} else {1};
    let gid = match matches.opt_str("reference") {
        Some(fref) => {
            let mut stat : libc::stat = unsafe { mem::uninitialized() };
            let statres = unsafe { libc::stat(fref.as_ptr() as *const _, &mut stat as *mut libc::stat) };
            if statres == 0 {
                stat.st_gid
            } else {
                crash!(1, "{}", Error::last_os_error())
            }
        },
        None => {
            match get_group(matches.free[0].as_ref()) {
                Some(grp) => grp.gr_gid,
                None => crash!(1, "invalid group: ‘{}’", matches.free[0])
            }
        }
    };
    chgrp(
        gid,
        matches.opt_present("recursive"),
        matches.opt_present("dereference"),
        &Verbosity::from_matches(&matches),
        matches.opt_present("preserve-root"),
        &matches.free[free_first_file_index..matches.free.len()]
    )
}

pub fn chgrp_file(
    gid: gid_t,
    path: &Path,
    verbosity: &Verbosity,
    exit_code: &mut i32
) -> Result<()>{
    let c_path = path.as_os_str().to_cstring().unwrap();
    if unsafe { chown(c_path.as_ptr(), !0u32, gid) } == 0 {
        Ok(())
    } else {
        *exit_code = 1;
        show_error!("{}", Error::last_os_error());
        Err(Error::last_os_error())
    }
}

pub fn do_chgrp(
    gid: gid_t,
    path: &Path,
    recursive: bool,
    dereference_symlinks: bool,
    verbosity: &Verbosity,
    preserve_root: bool,
    exit_code: &mut i32
) {
    if recursive {
        for entry in WalkDir::new(path).follow_links(dereference_symlinks){
            let entry = entry.unwrap();
            let entry_path = entry.path();
            let result = chgrp_file(gid, entry_path, verbosity, exit_code);
            if entry.path().is_dir() {
                do_chgrp(
                    gid,
                    entry_path,
                    recursive,
                    dereference_symlinks,
                    verbosity,
                    preserve_root,
                    exit_code
                )
            }
        }

    } else {
         let result = chgrp_file(
             gid,
             path,
             verbosity,
             exit_code
         );
    }
}

pub fn chgrp(
    gid: gid_t,
    recursive: bool,
    dereference_symlinks: bool,
    verbosity: &Verbosity,
    preserve_root: bool,
    filenames: &[String]
) -> i32 {
    let mut exit_code = 0i32;
    for filename in filenames{
        let path = Path::new(filename);
        do_chgrp(
            gid,
            path,
            recursive,
            dereference_symlinks,
            &verbosity,
            preserve_root,
            &mut exit_code
        )
    };
    exit_code

}
