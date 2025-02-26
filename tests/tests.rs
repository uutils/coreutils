// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(feature = "arch")]
#[path = "by-util/test_arch.rs"]
mod test_arch;

#[cfg(feature = "base32")]
#[path = "by-util/test_base32.rs"]
mod test_base32;

#[cfg(feature = "base64")]
#[path = "by-util/test_base64.rs"]
mod test_base64;

#[cfg(feature = "basename")]
#[path = "by-util/test_basename.rs"]
mod test_basename;

#[cfg(feature = "basenc")]
#[path = "by-util/test_basenc.rs"]
mod test_basenc;

#[cfg(feature = "cat")]
#[path = "by-util/test_cat.rs"]
mod test_cat;

#[cfg(feature = "chcon")]
#[path = "by-util/test_chcon.rs"]
mod test_chcon;

#[cfg(feature = "chgrp")]
#[path = "by-util/test_chgrp.rs"]
mod test_chgrp;

#[cfg(feature = "chmod")]
#[path = "by-util/test_chmod.rs"]
mod test_chmod;

#[cfg(feature = "chown")]
#[path = "by-util/test_chown.rs"]
mod test_chown;

#[cfg(feature = "chroot")]
#[path = "by-util/test_chroot.rs"]
mod test_chroot;

#[cfg(feature = "cksum")]
#[path = "by-util/test_cksum.rs"]
mod test_cksum;

#[cfg(feature = "comm")]
#[path = "by-util/test_comm.rs"]
mod test_comm;

#[cfg(feature = "cp")]
#[path = "by-util/test_cp.rs"]
mod test_cp;

#[cfg(feature = "csplit")]
#[path = "by-util/test_csplit.rs"]
mod test_csplit;

#[cfg(feature = "cut")]
#[path = "by-util/test_cut.rs"]
mod test_cut;

#[cfg(feature = "date")]
#[path = "by-util/test_date.rs"]
mod test_date;

#[cfg(feature = "dd")]
#[path = "by-util/test_dd.rs"]
mod test_dd;

#[cfg(feature = "df")]
#[path = "by-util/test_df.rs"]
mod test_df;

#[cfg(feature = "dir")]
#[path = "by-util/test_dir.rs"]
mod test_dir;

#[cfg(feature = "dircolors")]
#[path = "by-util/test_dircolors.rs"]
mod test_dircolors;

#[cfg(feature = "dirname")]
#[path = "by-util/test_dirname.rs"]
mod test_dirname;

#[cfg(feature = "du")]
#[path = "by-util/test_du.rs"]
mod test_du;

#[cfg(feature = "echo")]
#[path = "by-util/test_echo.rs"]
mod test_echo;

#[cfg(feature = "env")]
#[path = "by-util/test_env.rs"]
mod test_env;

#[cfg(feature = "expand")]
#[path = "by-util/test_expand.rs"]
mod test_expand;

#[cfg(feature = "expr")]
#[path = "by-util/test_expr.rs"]
mod test_expr;

#[cfg(feature = "factor")]
#[path = "by-util/test_factor.rs"]
mod test_factor;

#[cfg(feature = "false")]
#[path = "by-util/test_false.rs"]
mod test_false;

#[cfg(feature = "fmt")]
#[path = "by-util/test_fmt.rs"]
mod test_fmt;

#[cfg(feature = "fold")]
#[path = "by-util/test_fold.rs"]
mod test_fold;

#[cfg(feature = "groups")]
#[path = "by-util/test_groups.rs"]
mod test_groups;

#[cfg(feature = "hashsum")]
#[path = "by-util/test_hashsum.rs"]
mod test_hashsum;

#[cfg(feature = "head")]
#[path = "by-util/test_head.rs"]
mod test_head;

#[cfg(feature = "hostid")]
#[path = "by-util/test_hostid.rs"]
mod test_hostid;

#[cfg(feature = "hostname")]
#[path = "by-util/test_hostname.rs"]
mod test_hostname;

#[cfg(feature = "id")]
#[path = "by-util/test_id.rs"]
mod test_id;

#[cfg(feature = "install")]
#[path = "by-util/test_install.rs"]
mod test_install;

#[cfg(feature = "join")]
#[path = "by-util/test_join.rs"]
mod test_join;

#[cfg(feature = "kill")]
#[path = "by-util/test_kill.rs"]
mod test_kill;

#[cfg(feature = "link")]
#[path = "by-util/test_link.rs"]
mod test_link;

#[cfg(feature = "ln")]
#[path = "by-util/test_ln.rs"]
mod test_ln;

#[cfg(feature = "logname")]
#[path = "by-util/test_logname.rs"]
mod test_logname;

#[cfg(feature = "ls")]
#[path = "by-util/test_ls.rs"]
mod test_ls;

#[cfg(feature = "mkdir")]
#[path = "by-util/test_mkdir.rs"]
mod test_mkdir;

#[cfg(feature = "mkfifo")]
#[path = "by-util/test_mkfifo.rs"]
mod test_mkfifo;

#[cfg(feature = "mknod")]
#[path = "by-util/test_mknod.rs"]
mod test_mknod;

#[cfg(feature = "mktemp")]
#[path = "by-util/test_mktemp.rs"]
mod test_mktemp;

#[cfg(feature = "more")]
#[path = "by-util/test_more.rs"]
mod test_more;

#[cfg(feature = "mv")]
#[path = "by-util/test_mv.rs"]
mod test_mv;

#[cfg(feature = "nice")]
#[path = "by-util/test_nice.rs"]
mod test_nice;

#[cfg(feature = "nl")]
#[path = "by-util/test_nl.rs"]
mod test_nl;

#[cfg(feature = "nohup")]
#[path = "by-util/test_nohup.rs"]
mod test_nohup;

#[cfg(feature = "nproc")]
#[path = "by-util/test_nproc.rs"]
mod test_nproc;

#[cfg(feature = "numfmt")]
#[path = "by-util/test_numfmt.rs"]
mod test_numfmt;

#[cfg(feature = "od")]
#[path = "by-util/test_od.rs"]
mod test_od;

#[cfg(feature = "paste")]
#[path = "by-util/test_paste.rs"]
mod test_paste;

#[cfg(feature = "pathchk")]
#[path = "by-util/test_pathchk.rs"]
mod test_pathchk;

#[cfg(feature = "pinky")]
#[path = "by-util/test_pinky.rs"]
mod test_pinky;

#[cfg(feature = "pr")]
#[path = "by-util/test_pr.rs"]
mod test_pr;

#[cfg(feature = "printenv")]
#[path = "by-util/test_printenv.rs"]
mod test_printenv;

#[cfg(feature = "printf")]
#[path = "by-util/test_printf.rs"]
mod test_printf;

#[cfg(feature = "ptx")]
#[path = "by-util/test_ptx.rs"]
mod test_ptx;

#[cfg(feature = "pwd")]
#[path = "by-util/test_pwd.rs"]
mod test_pwd;

#[cfg(feature = "readlink")]
#[path = "by-util/test_readlink.rs"]
mod test_readlink;

#[cfg(feature = "realpath")]
#[path = "by-util/test_realpath.rs"]
mod test_realpath;

#[cfg(feature = "rm")]
#[path = "by-util/test_rm.rs"]
mod test_rm;

#[cfg(feature = "rmdir")]
#[path = "by-util/test_rmdir.rs"]
mod test_rmdir;

#[cfg(feature = "runcon")]
#[path = "by-util/test_runcon.rs"]
mod test_runcon;

#[cfg(feature = "seq")]
#[path = "by-util/test_seq.rs"]
mod test_seq;

#[cfg(feature = "shred")]
#[path = "by-util/test_shred.rs"]
mod test_shred;

#[cfg(feature = "shuf")]
#[path = "by-util/test_shuf.rs"]
mod test_shuf;

#[cfg(feature = "sleep")]
#[path = "by-util/test_sleep.rs"]
mod test_sleep;

#[cfg(feature = "sort")]
#[path = "by-util/test_sort.rs"]
mod test_sort;

#[cfg(feature = "split")]
#[path = "by-util/test_split.rs"]
mod test_split;

#[cfg(feature = "stat")]
#[path = "by-util/test_stat.rs"]
mod test_stat;

#[cfg(feature = "stdbuf")]
#[path = "by-util/test_stdbuf.rs"]
mod test_stdbuf;

#[cfg(feature = "stty")]
#[path = "by-util/test_stty.rs"]
mod test_stty;

#[cfg(feature = "sum")]
#[path = "by-util/test_sum.rs"]
mod test_sum;

#[cfg(feature = "sync")]
#[path = "by-util/test_sync.rs"]
mod test_sync;

#[cfg(feature = "tac")]
#[path = "by-util/test_tac.rs"]
mod test_tac;

#[cfg(feature = "tail")]
#[path = "by-util/test_tail.rs"]
mod test_tail;

#[cfg(feature = "tee")]
#[path = "by-util/test_tee.rs"]
mod test_tee;

#[cfg(feature = "test")]
#[path = "by-util/test_test.rs"]
mod test_test;

#[cfg(feature = "timeout")]
#[path = "by-util/test_timeout.rs"]
mod test_timeout;

#[cfg(feature = "touch")]
#[path = "by-util/test_touch.rs"]
mod test_touch;

#[cfg(feature = "tr")]
#[path = "by-util/test_tr.rs"]
mod test_tr;

#[cfg(feature = "true")]
#[path = "by-util/test_true.rs"]
mod test_true;

#[cfg(feature = "truncate")]
#[path = "by-util/test_truncate.rs"]
mod test_truncate;

#[cfg(feature = "tsort")]
#[path = "by-util/test_tsort.rs"]
mod test_tsort;

#[cfg(feature = "tty")]
#[path = "by-util/test_tty.rs"]
mod test_tty;

#[cfg(feature = "uname")]
#[path = "by-util/test_uname.rs"]
mod test_uname;

#[cfg(feature = "unexpand")]
#[path = "by-util/test_unexpand.rs"]
mod test_unexpand;

#[cfg(feature = "uniq")]
#[path = "by-util/test_uniq.rs"]
mod test_uniq;

#[cfg(feature = "unlink")]
#[path = "by-util/test_unlink.rs"]
mod test_unlink;

#[cfg(feature = "uptime")]
#[path = "by-util/test_uptime.rs"]
mod test_uptime;

#[cfg(feature = "users")]
#[path = "by-util/test_users.rs"]
mod test_users;

#[cfg(feature = "vdir")]
#[path = "by-util/test_vdir.rs"]
mod test_vdir;

#[cfg(feature = "wc")]
#[path = "by-util/test_wc.rs"]
mod test_wc;

#[cfg(feature = "who")]
#[path = "by-util/test_who.rs"]
mod test_who;

#[cfg(feature = "whoami")]
#[path = "by-util/test_whoami.rs"]
mod test_whoami;

#[cfg(feature = "yes")]
#[path = "by-util/test_yes.rs"]
mod test_yes;
