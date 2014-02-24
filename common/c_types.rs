#[allow(dead_code)];

use std::libc::{
    c_char,
    c_int,
    time_t
};

pub struct c_passwd {
    pw_name:    *c_char,    /* user name */
    pw_passwd:  *c_char,    /* user name */
    pw_uid:     c_int,      /* user uid */
    pw_gid:     c_int,      /* user gid */
    pw_change:  time_t,
    pw_class:   *c_char,
    pw_gecos:   *c_char,
    pw_dir:     *c_char,
    pw_shell:   *c_char,
    pw_expire:  time_t
}

pub struct c_group {
    gr_name: *c_char /* group name */
}
