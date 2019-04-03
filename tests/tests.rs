#[macro_use]
mod common;

#[cfg(unix)]
#[macro_use]
extern crate lazy_static;

#[cfg(unix)]
extern crate rust_users;

// For conditional compilation
macro_rules! unix_only {
    ($($fea:expr, $m:ident);+) => {
        $(
            #[cfg(unix)]
            #[cfg(feature = $fea)]
            mod $m;
         )+
    };
}
unix_only! {
    "chmod", test_chmod;
    "chown", test_chown;
    "chgrp", test_chgrp;
    "install", test_install;
    "pathchk", test_pathchk;
    "pinky", test_pinky;
    "stdbuf", test_stdbuf;
    "touch", test_touch;
    "unlink", test_unlink;
    "who", test_who;
    // Be aware of the trailing semicolon after the last item
    "stat", test_stat
}

macro_rules! generic {
    ($($fea:expr, $m:ident);+) => {
        $(
            #[cfg(feature = $fea)]
            mod $m;
         )+
    };
}
generic! {
    "base32", test_base32;
    "base64", test_base64;
    "basename", test_basename;
    "cat", test_cat;
    "cksum", test_cksum;
    "comm", test_comm;
    "cp", test_cp;
    "cut", test_cut;
    "dircolors", test_dircolors;
    "dirname", test_dirname;
    "du", test_du;
    "echo", test_echo;
    "env", test_env;
    "expr", test_expr;
    "factor", test_factor;
    "false", test_false;
    "fold", test_fold;
    "hashsum", test_hashsum;
    "head", test_head;
    "join", test_join;
    "link", test_link;
    "ln", test_ln;
    "ls", test_ls;
    "mkdir", test_mkdir;
    "mktemp", test_mktemp;
    "mv", test_mv;
    "numfmt", test_numfmt;
    "nl", test_nl;
    "od", test_od;
    "paste", test_paste;
    "printf", test_printf;
    "ptx", test_ptx;
    "pwd", test_pwd;
    "readlink", test_readlink;
    "realpath", test_realpath;
    "rm", test_rm;
    "rmdir", test_rmdir;
    "seq", test_seq;
    "sort", test_sort;
    "split", test_split;
    "sum", test_sum;
    "tac", test_tac;
    "tail", test_tail;
    "test", test_test;
    "tr", test_tr;
    "true", test_true;
    "truncate", test_truncate;
    "tsort", test_tsort;
    "unexpand", test_unexpand;
    "uniq", test_uniq;
    "wc", test_wc;
    // Be aware of the trailing semicolon after the last item
    "hostname", test_hostname
}
