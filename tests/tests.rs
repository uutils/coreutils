#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(choose))]

#[macro_use]
mod common;

choose! {
    unix => {
        test_chmod
        test_chown
        test_chgrp
        test_install
        test_mv
        test_pathchk
        test_pinky
        test_stdbuf
        test_touch
        test_unlink
        test_who
        test_stat
    }
    generic => {
        test_base32
        test_base64
        test_basename
        test_cat
        test_cksum
        test_comm
        test_cp
        test_cut
        test_dircolors
        test_dirname
        test_echo
        test_env
        test_expr
        test_factor
        test_false
        test_fold
        test_hashsum
        test_head
        test_link
        test_ln
        test_ls
        test_mkdir
        test_mktemp
        test_nl
        test_od
        test_paste
        test_printf
        test_ptx
        test_pwd
        test_readlink
        test_realpath
        test_rm
        test_rmdir
        test_seq
        test_sort
        test_split
        test_sum
        test_tac
        test_tail
        test_test
        test_tr
        test_true
        test_truncate
        test_tsort
        test_unexpand
        test_uniq
        test_wc
    }
}
