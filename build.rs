// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) krate manpages mangen
use clap::Command;
use clap_complete::{generate_to, shells};
use clap_mangen::Man;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

include!("./src/args.rs");

macro_rules! collect_functions {
    ($($module:ident),*) => {{
        let mut map: HashMap<&'static str, fn() -> Command> = HashMap::new();
        $(
            map.insert(stringify!($module), $module::uu_app);
        )*
        map.insert("md5sum", uu_hashsum::uu_app_common);
        map.insert("sha1sum", uu_hashsum::uu_app_common);
        map.insert("sha224sum", uu_hashsum::uu_app_common);
        map.insert("sha256sum", uu_hashsum::uu_app_common);
        map.insert("sha384sum", uu_hashsum::uu_app_common);
        map.insert("sha512sum", uu_hashsum::uu_app_common);
        map.insert("sha3sum", uu_hashsum::uu_app_bits);
        map.insert("sha3-224sum", uu_hashsum::uu_app_common);
        map.insert("sha3-256sum", uu_hashsum::uu_app_common);
        map.insert("sha3-384sum", uu_hashsum::uu_app_common);
        map.insert("sha3-512sum", uu_hashsum::uu_app_common);
        map.insert("shake128sum", uu_hashsum::uu_app_bits);
        map.insert("shake256sum", uu_hashsum::uu_app_bits);
        map.insert("b2sum", uu_hashsum::uu_app_common);
        map.insert("b3sum", uu_hashsum::uu_app_b3sum);
        map
    }};
}

/// # Errors
/// Returns an error if the manpage generation fails.
#[allow(clippy::too_many_lines)]
pub fn generate_manpages(_crates: &[String]) -> Result<(), std::io::Error> {
    let crates = collect_functions!(
        uu_arch,
        uu_base32,
        uu_base64,
        uu_basename,
        uu_basenc,
        uu_cat,
        uu_chcon,
        uu_chgrp,
        uu_chmod,
        uu_chown,
        uu_chroot,
        uu_cksum,
        uu_comm,
        uu_cp,
        uu_csplit,
        uu_cut,
        uu_date,
        uu_dd,
        uu_df,
        // uu_dir, // TODO
        uu_dircolors,
        uu_dirname,
        uu_du,
        uu_echo,
        uu_env,
        uu_expand,
        uu_expr,
        uu_factor,
        uu_false,
        uu_fmt,
        uu_fold,
        uu_groups,
        // uu_hashsum, // Done in macro
        uu_head,
        uu_hostid,
        uu_hostname,
        uu_id,
        uu_install,
        uu_join,
        uu_kill,
        uu_link,
        uu_ln,
        uu_logname,
        uu_ls,
        uu_mkdir,
        uu_mkfifo,
        uu_mknod,
        uu_mktemp,
        uu_more,
        uu_mv,
        uu_nice,
        uu_nl,
        uu_nohup,
        uu_nproc,
        uu_numfmt,
        uu_od,
        uu_paste,
        uu_pathchk,
        uu_pinky,
        uu_pr,
        uu_printenv,
        uu_printf,
        uu_ptx,
        uu_pwd,
        uu_readlink,
        uu_realpath,
        uu_rm,
        uu_rmdir,
        uu_runcon,
        uu_seq,
        uu_shred,
        uu_shuf,
        uu_sleep,
        uu_sort,
        uu_split,
        uu_stat,
        uu_stdbuf,
        uu_stty,
        uu_sum,
        uu_sync,
        uu_tac,
        uu_tail,
        uu_tee,
        uu_test,
        uu_timeout,
        uu_touch,
        uu_tr,
        uu_true,
        uu_truncate,
        uu_tsort,
        uu_tty,
        uu_uname,
        uu_unexpand,
        uu_uniq,
        uu_unlink,
        uu_uptime,
        uu_users,
        // uu_vdir, // TODO
        uu_wc,
        uu_who,
        uu_whoami,
        uu_yes
    );
    let out_dir_completion = "completion";
    std::fs::create_dir_all(out_dir_completion)?;
    let out_dir_manpages = "man-page";
    std::fs::create_dir_all(out_dir_manpages)?;

    for (app_name, args_fn) in crates {
        let mut cmd = args_fn().name(app_name);

        generate_to(shells::Bash, &mut cmd, app_name, out_dir_completion)?;
        generate_to(shells::Zsh, &mut cmd, app_name, out_dir_completion)?;
        generate_to(shells::Fish, &mut cmd, app_name, out_dir_completion)?;
        generate_to(shells::PowerShell, &mut cmd, app_name, out_dir_completion)?;
        generate_to(shells::Elvish, &mut cmd, app_name, out_dir_completion)?;

        let file = Path::new(out_dir_manpages).join(app_name.to_owned() + ".1");
        let mut file = File::create(file)?;
        Man::new(cmd).render(&mut file)?;
    }
    Ok(())
}

pub fn main() {
    const ENV_FEATURE_PREFIX: &str = "CARGO_FEATURE_";
    const FEATURE_PREFIX: &str = "feat_";
    const OVERRIDE_PREFIX: &str = "uu_";

    if let Ok(profile) = env::var("PROFILE") {
        println!("cargo:rustc-cfg=build={profile:?}");
    }

    let out_dir = env::var("OUT_DIR").unwrap();

    let mut crates = Vec::new();
    for (key, val) in env::vars() {
        if val == "1" && key.starts_with(ENV_FEATURE_PREFIX) {
            let krate = key[ENV_FEATURE_PREFIX.len()..].to_lowercase();
            // Allow this as we have a bunch of info in the comments
            #[allow(clippy::match_same_arms)]
            match krate.as_ref() {
                "default" | "macos" | "unix" | "windows" | "selinux" | "zip" => continue, // common/standard feature names
                "nightly" | "test_unimplemented" | "expensive_tests" => continue, // crate-local custom features
                "uudoc" => continue, // is not a utility
                "test" => continue, // over-ridden with 'uu_test' to avoid collision with rust core crate 'test'
                s if s.starts_with(FEATURE_PREFIX) => continue, // crate feature sets
                _ => {}             // util feature name
            }
            crates.push(krate);
        }
    }
    crates.sort();
    generate_manpages(&crates).unwrap();

    let mut mf = File::create(Path::new(&out_dir).join("uutils_map.rs")).unwrap();

    mf.write_all(
        "type UtilityMap<T> = phf::OrderedMap<&'static str, (fn(T) -> i32, fn() -> Command)>;\n\
         \n\
         #[allow(clippy::too_many_lines)]
         #[allow(clippy::unreadable_literal)]
         fn util_map<T: uucore::Args>() -> UtilityMap<T> {\n"
            .as_bytes(),
    )
    .unwrap();

    let mut phf_map = phf_codegen::OrderedMap::<&str>::new();
    for krate in &crates {
        let map_value = format!("({krate}::uumain, {krate}::uu_app)");
        match krate.as_ref() {
            // 'test' is named uu_test to avoid collision with rust core crate 'test'.
            // It can also be invoked by name '[' for the '[ expr ] syntax'.
            "uu_test" => {
                phf_map.entry("test", &map_value);
                phf_map.entry("[", &map_value);
            }
            k if k.starts_with(OVERRIDE_PREFIX) => {
                phf_map.entry(&k[OVERRIDE_PREFIX.len()..], &map_value);
            }
            "false" | "true" => {
                phf_map.entry(krate, &format!("(r#{krate}::uumain, r#{krate}::uu_app)"));
            }
            "hashsum" => {
                phf_map.entry(krate, &format!("({krate}::uumain, {krate}::uu_app_custom)"));

                let map_value = format!("({krate}::uumain, {krate}::uu_app_common)");
                let map_value_bits = format!("({krate}::uumain, {krate}::uu_app_bits)");
                let map_value_b3sum = format!("({krate}::uumain, {krate}::uu_app_b3sum)");
                phf_map.entry("md5sum", &map_value);
                phf_map.entry("sha1sum", &map_value);
                phf_map.entry("sha224sum", &map_value);
                phf_map.entry("sha256sum", &map_value);
                phf_map.entry("sha384sum", &map_value);
                phf_map.entry("sha512sum", &map_value);
                phf_map.entry("sha3sum", &map_value_bits);
                phf_map.entry("sha3-224sum", &map_value);
                phf_map.entry("sha3-256sum", &map_value);
                phf_map.entry("sha3-384sum", &map_value);
                phf_map.entry("sha3-512sum", &map_value);
                phf_map.entry("shake128sum", &map_value_bits);
                phf_map.entry("shake256sum", &map_value_bits);
                phf_map.entry("b2sum", &map_value);
                phf_map.entry("b3sum", &map_value_b3sum);
            }
            _ => {
                phf_map.entry(krate, &map_value);
            }
        }
    }
    write!(mf, "{}", phf_map.build()).unwrap();
    mf.write_all(b"\n}\n").unwrap();

    mf.flush().unwrap();
}
