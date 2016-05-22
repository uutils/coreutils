extern crate filetime;
extern crate libc;
extern crate rand;
extern crate regex;
extern crate tempdir;
extern crate time;
extern crate uu_tail;

#[cfg(windows)] extern crate kernel32;
#[cfg(windows)] extern crate winapi;

#[macro_use]
mod common;

#[path="../src/factor/sieve.rs"]
mod sieve;

#[cfg(unix)] mod test_chmod;
#[cfg(unix)] mod test_mv;
#[cfg(unix)] mod test_pathchk;
#[cfg(unix)] mod test_stdbuf;
#[cfg(unix)] mod test_touch;
#[cfg(unix)] mod test_unlink;

mod test_base64;
mod test_basename;
mod test_cat;
mod test_cksum;
mod test_comm;
mod test_cp;
mod test_cut;
mod test_dircolors;
mod test_dirname;
mod test_echo;
mod test_env;
mod test_expr;
mod test_factor;
mod test_false;
mod test_fold;
mod test_hashsum;
mod test_head;
mod test_link;
mod test_ln;
mod test_ls;
mod test_mkdir;
mod test_mktemp;
mod test_nl;
mod test_od;
mod test_paste;
mod test_printf;
mod test_ptx;
mod test_pwd;
mod test_readlink;
mod test_realpath;
mod test_rm;
mod test_rmdir;
mod test_seq;
mod test_sort;
mod test_split;
mod test_sum;
mod test_tac;
mod test_tail;
mod test_test;
mod test_tr;
mod test_true;
mod test_truncate;
mod test_tsort;
mod test_unexpand;
mod test_uniq;
mod test_wc;
