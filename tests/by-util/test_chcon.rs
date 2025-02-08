// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (jargon) xattributes
#![allow(clippy::missing_errors_doc, clippy::similar_names)]
#![cfg(feature = "feat_selinux")]

use std::ffi::CString;
use std::path::Path;
use std::{io, iter, str};

use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn version() {
    new_ucmd!().arg("--version").succeeds();
    new_ucmd!().arg("-V").succeeds();
}

#[test]
fn help() {
    new_ucmd!().fails();
    new_ucmd!().arg("--help").succeeds();
    new_ucmd!().arg("-h").fails(); // -h is NOT --help, it is actually --no-dereference.
}

#[test]
fn reference_errors() {
    for args in [
        &["--verbose", "--reference"] as &[&str],
        &["--verbose", "--reference=/dev/null"],
        &["--verbose", "--reference=/inexistent", "/dev/null"],
    ] {
        new_ucmd!().args(args).fails();
    }
}

#[test]
fn recursive_errors() {
    for args in [
        &["--verbose", "-P"] as &[&str],
        &["--verbose", "-H"],
        &["--verbose", "-L"],
        &["--verbose", "--recursive", "-P", "--dereference"],
        &["--verbose", "--recursive", "-H", "--no-dereference"],
        &["--verbose", "--recursive", "-L", "--no-dereference"],
    ] {
        new_ucmd!().args(args).fails();
    }
}

#[test]
fn valid_context() {
    let (dir, mut cmd) = at_and_ucmd!();
    dir.touch("a.tmp");
    dir.symlink_file("a.tmp", "la.tmp");

    let la_context = get_file_context(dir.plus("a.tmp")).unwrap();
    let new_la_context = "guest_u:object_r:etc_t:s0:c42";

    cmd.args(&["--verbose", new_la_context])
        .arg(dir.plus("la.tmp"))
        .succeeds();
    assert_eq!(get_file_context(dir.plus("la.tmp")).unwrap(), la_context);
    assert_eq!(
        get_file_context(dir.plus("a.tmp")).unwrap().as_deref(),
        Some(new_la_context)
    );
}

#[test]
fn valid_context_on_valid_symlink() {
    let (dir, mut cmd) = at_and_ucmd!();
    dir.touch("a.tmp");
    dir.symlink_file("a.tmp", "la.tmp");

    let a_context = get_file_context(dir.plus("a.tmp")).unwrap();
    let new_la_context = "guest_u:object_r:etc_t:s0:c42";

    cmd.args(&["--verbose", "--no-dereference", new_la_context])
        .arg(dir.plus("la.tmp"))
        .succeeds();
    assert_eq!(
        get_file_context(dir.plus("la.tmp")).unwrap().as_deref(),
        Some(new_la_context)
    );
    assert_eq!(get_file_context(dir.plus("a.tmp")).unwrap(), a_context);
}

#[test]
fn valid_context_on_valid_symlink_override_dereference() {
    let (dir, mut cmd) = at_and_ucmd!();
    dir.touch("a.tmp");
    dir.symlink_file("a.tmp", "la.tmp");

    let a_context = get_file_context(dir.plus("a.tmp")).unwrap();
    let la_context = get_file_context(dir.plus("la.tmp")).unwrap();
    let new_a_context = "guest_u:object_r:etc_t:s0:c42";
    assert_ne!(a_context.as_deref(), Some(new_a_context));
    assert_ne!(la_context.as_deref(), Some(new_a_context));

    cmd.args(&[
        "--verbose",
        "--no-dereference",
        "--dereference",
        new_a_context,
    ])
    .arg(dir.plus("la.tmp"))
    .succeeds();
    assert_eq!(
        get_file_context(dir.plus("a.tmp")).unwrap().as_deref(),
        Some(new_a_context)
    );
    assert_eq!(get_file_context(dir.plus("la.tmp")).unwrap(), la_context);
}

#[test]
fn valid_context_on_broken_symlink() {
    let (dir, mut cmd) = at_and_ucmd!();
    dir.symlink_file("a.tmp", "la.tmp");

    let new_la_context = "guest_u:object_r:etc_t:s0:c42";

    cmd.args(&["--verbose", "--no-dereference", new_la_context])
        .arg(dir.plus("la.tmp"))
        .succeeds();
    assert_eq!(
        get_file_context(dir.plus("la.tmp")).unwrap().as_deref(),
        Some(new_la_context)
    );
}

#[test]
fn valid_context_on_broken_symlink_after_deref() {
    let (dir, mut cmd) = at_and_ucmd!();
    dir.symlink_file("a.tmp", "la.tmp");

    let la_context = get_file_context(dir.plus("la.tmp")).unwrap();
    let new_la_context = "guest_u:object_r:etc_t:s0:c42";
    assert_ne!(la_context.as_deref(), Some(new_la_context));

    cmd.args(&[
        "--verbose",
        "--dereference",
        "--no-dereference",
        new_la_context,
    ])
    .arg(dir.plus("la.tmp"))
    .succeeds();
    assert_eq!(
        get_file_context(dir.plus("la.tmp")).unwrap().as_deref(),
        Some(new_la_context)
    );
}

#[test]
fn valid_context_with_prior_xattributes() {
    let (dir, mut cmd) = at_and_ucmd!();
    dir.touch("a.tmp");

    let a_context = get_file_context(dir.plus("a.tmp")).unwrap();
    if a_context.is_none() {
        set_file_context(dir.plus("a.tmp"), "unconfined_u:object_r:user_tmp_t:s0").unwrap();
    }
    let new_la_context = "guest_u:object_r:etc_t:s0:c42";

    cmd.args(&["--verbose", new_la_context])
        .arg(dir.plus("a.tmp"))
        .succeeds();
    assert_eq!(
        get_file_context(dir.plus("a.tmp")).unwrap().as_deref(),
        Some(new_la_context)
    );
}

#[test]
fn valid_context_directory() {
    let (dir, mut cmd) = at_and_ucmd!();
    dir.mkdir("a");
    dir.symlink_dir("a", "la");

    let b_path = Path::new("a").join("b.txt");
    dir.touch(b_path.to_str().unwrap());

    let la_context = get_file_context(dir.plus("la")).unwrap();
    let b_context = get_file_context(dir.plus(b_path.to_str().unwrap())).unwrap();

    let new_la_context = "guest_u:object_r:etc_t:s0:c42";

    cmd.args(&["--verbose", new_la_context])
        .arg(dir.plus("la"))
        .succeeds();
    assert_eq!(get_file_context(dir.plus("la")).unwrap(), la_context);
    assert_eq!(
        get_file_context(dir.plus("a")).unwrap().as_deref(),
        Some(new_la_context)
    );
    assert_eq!(
        get_file_context(dir.plus(b_path.to_str().unwrap())).unwrap(),
        b_context
    );
}

#[test]
fn valid_context_directory_recursive() {
    let (dir, mut cmd) = at_and_ucmd!();
    dir.mkdir("a");
    dir.symlink_dir("a", "la");

    let b_path = Path::new("a").join("b.txt");
    dir.touch(b_path.to_str().unwrap());

    let a_context = get_file_context(dir.plus("a")).unwrap();
    let b_context = get_file_context(dir.plus(b_path.to_str().unwrap())).unwrap();

    let new_la_context = "guest_u:object_r:etc_t:s0:c42";

    // -P (default): do not traverse any symbolic links.
    cmd.args(&["--verbose", "--recursive", new_la_context])
        .arg(dir.plus("la"))
        .succeeds();
    assert_eq!(
        get_file_context(dir.plus("la")).unwrap().as_deref(),
        Some(new_la_context)
    );
    assert_eq!(get_file_context(dir.plus("a")).unwrap(), a_context);
    assert_eq!(
        get_file_context(dir.plus(b_path.to_str().unwrap())).unwrap(),
        b_context
    );
}

#[test]
fn valid_context_directory_recursive_follow_args_dir_symlinks() {
    let (dir, mut cmd) = at_and_ucmd!();
    dir.mkdir("a");
    dir.symlink_dir("a", "la");

    let b_path = Path::new("a").join("b.txt");
    dir.touch(b_path.to_str().unwrap());

    let la_context = get_file_context(dir.plus("la")).unwrap();
    let new_la_context = "guest_u:object_r:etc_t:s0:c42";

    /*
    let lc_path = Path::new("a").join("lc");
    dir.symlink_dir("c", lc_path.to_str().unwrap());
    assert_eq!(
        get_file_context(dir.plus(lc_path.to_str().unwrap())).unwrap(),
        None
    );
    */

    // -H: if a command line argument is a symbolic link to a directory, traverse it.
    cmd.args(&["--verbose", "--recursive", "-H", new_la_context])
        .arg(dir.plus("la"))
        .succeeds();
    assert_eq!(
        get_file_context(dir.plus("a")).unwrap().as_deref(),
        Some(new_la_context)
    );
    assert_eq!(
        get_file_context(dir.plus(b_path.to_str().unwrap()))
            .unwrap()
            .as_deref(),
        Some(new_la_context)
    );
    assert_eq!(get_file_context(dir.plus("la")).unwrap(), la_context);
    /*
    assert_eq!(
        get_file_context(dir.plus(lc_path.to_str().unwrap()))
            .unwrap()
            .as_deref(),
        Some(new_la_context)
    );
    */
}

#[test]
fn valid_context_directory_recursive_follow_all_symlinks() {
    let (dir, mut cmd) = at_and_ucmd!();
    dir.mkdir("a");
    dir.symlink_dir("a", "la");

    let b_path = Path::new("a").join("b.txt");
    dir.touch(b_path.to_str().unwrap());

    let c_path = Path::new("a").join("c");
    dir.touch(c_path.to_str().unwrap());

    let lc_path = Path::new("a").join("lc");
    dir.symlink_dir(c_path.to_str().unwrap(), lc_path.to_str().unwrap());

    let la_context = get_file_context(dir.plus("la")).unwrap();
    let lc_context = get_file_context(dir.plus(lc_path.to_str().unwrap())).unwrap();

    let new_la_context = "guest_u:object_r:etc_t:s0:c42";

    // -L: traverse every symbolic link to a directory encountered.
    cmd.args(&["--verbose", "--recursive", "-L", new_la_context])
        .arg(dir.plus("la"))
        .succeeds();
    assert_eq!(get_file_context(dir.plus("la")).unwrap(), la_context);
    assert_eq!(
        get_file_context(dir.plus("a")).unwrap().as_deref(),
        Some(new_la_context)
    );
    assert_eq!(
        get_file_context(dir.plus(b_path.to_str().unwrap()))
            .unwrap()
            .as_deref(),
        Some(new_la_context)
    );
    assert_eq!(
        get_file_context(dir.plus(lc_path.to_str().unwrap())).unwrap(),
        lc_context
    );
    assert_eq!(
        get_file_context(dir.plus(c_path.to_str().unwrap()))
            .unwrap()
            .as_deref(),
        Some(new_la_context)
    );
}

#[test]
fn user_role_range_type() {
    let (dir, mut cmd) = at_and_ucmd!();

    dir.touch("a.tmp");
    let a_context = get_file_context(dir.plus("a.tmp")).unwrap();
    if a_context.is_none() {
        set_file_context(dir.plus("a.tmp"), "unconfined_u:object_r:user_tmp_t:s0").unwrap();
    }

    cmd.args(&[
        "--verbose",
        "--user=guest_u",
        "--role=object_r",
        "--type=etc_t",
        "--range=s0:c42",
    ])
    .arg(dir.plus("a.tmp"))
    .succeeds();

    assert_eq!(
        get_file_context(dir.plus("a.tmp")).unwrap().as_deref(),
        Some("guest_u:object_r:etc_t:s0:c42")
    );
}

#[test]
fn user_change() {
    let (dir, mut cmd) = at_and_ucmd!();

    dir.touch("a.tmp");
    let a_context = get_file_context(dir.plus("a.tmp")).unwrap();
    let new_a_context = if let Some(a_context) = a_context {
        let mut components: Vec<_> = a_context.split(':').collect();
        components[0] = "guest_u";
        components.join(":")
    } else {
        set_file_context(dir.plus("a.tmp"), "unconfined_u:object_r:user_tmp_t:s0").unwrap();
        String::from("guest_u:object_r:user_tmp_t:s0")
    };

    cmd.args(&["--verbose", "--user=guest_u"])
        .arg(dir.plus("a.tmp"))
        .succeeds();
    assert_eq!(
        get_file_context(dir.plus("a.tmp")).unwrap(),
        Some(new_a_context)
    );
}

#[test]
fn user_change_repeated() {
    let (dir, mut cmd) = at_and_ucmd!();

    dir.touch("a.tmp");
    let a_context = get_file_context(dir.plus("a.tmp")).unwrap();
    let new_a_context = if let Some(a_context) = a_context {
        let mut components: Vec<_> = a_context.split(':').collect();
        components[0] = "guest_u";
        components.join(":")
    } else {
        set_file_context(dir.plus("a.tmp"), "unconfined_u:object_r:user_tmp_t:s0").unwrap();
        String::from("guest_u:object_r:user_tmp_t:s0")
    };

    cmd.args(&["--verbose", "--user=wrong", "--user=guest_u"])
        .arg(dir.plus("a.tmp"))
        .succeeds();
    assert_eq!(
        get_file_context(dir.plus("a.tmp")).unwrap(),
        Some(new_a_context)
    );
}

#[test]
fn role_change() {
    let (dir, mut cmd) = at_and_ucmd!();

    dir.touch("a.tmp");
    let a_context = get_file_context(dir.plus("a.tmp")).unwrap();
    let new_a_context = if let Some(a_context) = a_context {
        let mut components: Vec<_> = a_context.split(':').collect();
        components[1] = "system_r";
        components.join(":")
    } else {
        set_file_context(dir.plus("a.tmp"), "unconfined_u:object_r:user_tmp_t:s0").unwrap();
        String::from("unconfined_u:system_r:user_tmp_t:s0")
    };

    cmd.args(&["--verbose", "--role=system_r"])
        .arg(dir.plus("a.tmp"))
        .succeeds();
    assert_eq!(
        get_file_context(dir.plus("a.tmp")).unwrap(),
        Some(new_a_context)
    );
}

#[test]
fn type_change() {
    let (dir, mut cmd) = at_and_ucmd!();

    dir.touch("a.tmp");
    let a_context = get_file_context(dir.plus("a.tmp")).unwrap();
    let new_a_context = if let Some(a_context) = a_context {
        let mut components: Vec<_> = a_context.split(':').collect();
        components[2] = "etc_t";
        components.join(":")
    } else {
        set_file_context(dir.plus("a.tmp"), "unconfined_u:object_r:user_tmp_t:s0").unwrap();
        String::from("unconfined_u:object_r:etc_t:s0")
    };

    cmd.args(&["--verbose", "--type=etc_t"])
        .arg(dir.plus("a.tmp"))
        .succeeds();
    assert_eq!(
        get_file_context(dir.plus("a.tmp")).unwrap(),
        Some(new_a_context)
    );
}

#[test]
fn range_change() {
    let (dir, mut cmd) = at_and_ucmd!();

    dir.touch("a.tmp");
    let a_context = get_file_context(dir.plus("a.tmp")).unwrap();
    let new_a_context = if let Some(a_context) = a_context {
        a_context
            .split(':')
            .take(3)
            .chain(iter::once("s0:c42"))
            .collect::<Vec<_>>()
            .join(":")
    } else {
        set_file_context(dir.plus("a.tmp"), "unconfined_u:object_r:user_tmp_t:s0").unwrap();
        String::from("unconfined_u:object_r:user_tmp_t:s0:c42")
    };

    cmd.args(&["--verbose", "--range=s0:c42"])
        .arg(dir.plus("a.tmp"))
        .succeeds();
    assert_eq!(
        get_file_context(dir.plus("a.tmp")).unwrap(),
        Some(new_a_context)
    );
}

#[test]
fn valid_reference() {
    let (dir, mut cmd) = at_and_ucmd!();

    dir.touch("a.tmp");
    let new_a_context = "guest_u:object_r:etc_t:s0:c42";
    set_file_context(dir.plus("a.tmp"), new_a_context).unwrap();

    dir.touch("b.tmp");
    let b_context = get_file_context(dir.plus("b.tmp")).unwrap();
    assert_ne!(b_context.as_deref(), Some(new_a_context));

    cmd.arg("--verbose")
        .arg(format!("--reference={}", dir.plus_as_string("a.tmp")))
        .arg(dir.plus("b.tmp"))
        .succeeds();
    assert_eq!(
        get_file_context(dir.plus("b.tmp")).unwrap().as_deref(),
        Some(new_a_context)
    );
}

#[test]
fn valid_reference_repeat_flags() {
    let (dir, mut cmd) = at_and_ucmd!();

    dir.touch("a.tmp");
    let new_a_context = "guest_u:object_r:etc_t:s0:c42";
    set_file_context(dir.plus("a.tmp"), new_a_context).unwrap();

    dir.touch("b.tmp");
    let b_context = get_file_context(dir.plus("b.tmp")).unwrap();
    assert_ne!(b_context.as_deref(), Some(new_a_context));

    cmd.arg("--verbose")
        .arg("-vvRRHHLLPP") // spell-checker:disable-line
        .arg("--no-preserve-root")
        .arg("--no-preserve-root")
        .arg("--preserve-root")
        .arg("--preserve-root")
        .arg("--dereference")
        .arg("--dereference")
        .arg("--no-dereference")
        .arg("--no-dereference")
        .arg(format!("--reference={}", dir.plus_as_string("a.tmp")))
        .arg(dir.plus("b.tmp"))
        .succeeds();
    assert_eq!(
        get_file_context(dir.plus("b.tmp")).unwrap().as_deref(),
        Some(new_a_context)
    );
}

#[test]
fn valid_reference_repeated_reference() {
    let (dir, mut cmd) = at_and_ucmd!();

    dir.touch("a.tmp");
    let new_a_context = "guest_u:object_r:etc_t:s0:c42";
    set_file_context(dir.plus("a.tmp"), new_a_context).unwrap();

    dir.touch("wrong.tmp");
    let new_wrong_context = "guest_u:object_r:etc_t:s42:c0";
    set_file_context(dir.plus("wrong.tmp"), new_wrong_context).unwrap();

    dir.touch("b.tmp");
    let b_context = get_file_context(dir.plus("b.tmp")).unwrap();
    assert_ne!(b_context.as_deref(), Some(new_a_context));

    cmd.arg("--verbose")
        .arg(format!("--reference={}", dir.plus_as_string("wrong.tmp")))
        .arg(format!("--reference={}", dir.plus_as_string("a.tmp")))
        .arg(dir.plus("b.tmp"))
        .succeeds();
    assert_eq!(
        get_file_context(dir.plus("b.tmp")).unwrap().as_deref(),
        Some(new_a_context)
    );
    assert_eq!(
        get_file_context(dir.plus("wrong.tmp")).unwrap().as_deref(),
        Some(new_wrong_context)
    );
}

#[test]
fn valid_reference_multi() {
    let (dir, mut cmd) = at_and_ucmd!();

    dir.touch("a.tmp");
    let new_a_context = "guest_u:object_r:etc_t:s0:c42";
    set_file_context(dir.plus("a.tmp"), new_a_context).unwrap();

    dir.touch("b1.tmp");
    let b1_context = get_file_context(dir.plus("b1.tmp")).unwrap();
    assert_ne!(b1_context.as_deref(), Some(new_a_context));

    dir.touch("b2.tmp");
    let b2_context = get_file_context(dir.plus("b2.tmp")).unwrap();
    assert_ne!(b2_context.as_deref(), Some(new_a_context));

    cmd.arg("--verbose")
        .arg(format!("--reference={}", dir.plus_as_string("a.tmp")))
        .arg(dir.plus("b1.tmp"))
        .arg(dir.plus("b2.tmp"))
        .succeeds();
    assert_eq!(
        get_file_context(dir.plus("b1.tmp")).unwrap().as_deref(),
        Some(new_a_context)
    );
    assert_eq!(
        get_file_context(dir.plus("b2.tmp")).unwrap().as_deref(),
        Some(new_a_context)
    );
}

fn get_file_context(path: impl AsRef<Path>) -> Result<Option<String>, selinux::errors::Error> {
    let path = path.as_ref();
    match selinux::SecurityContext::of_path(path, false, false) {
        Err(r) => {
            println!("get_file_context failed: '{}': {}.", path.display(), &r);
            Err(r)
        }

        Ok(None) => {
            println!(
                "get_file_context: '{}': No SELinux context defined.",
                path.display()
            );
            Ok(None)
        }

        Ok(Some(context)) => {
            let bytes = context
                .as_bytes()
                .splitn(2, |&b| b == 0_u8)
                .next()
                .unwrap_or_default();
            let context = String::from_utf8(bytes.into()).unwrap_or_default();
            println!("get_file_context: '{}' => '{}'.", context, path.display());
            Ok(Some(context))
        }
    }
}

fn set_file_context(path: impl AsRef<Path>, context: &str) -> Result<(), selinux::errors::Error> {
    let c_context = CString::new(context.as_bytes()).map_err(|_r| selinux::errors::Error::IO {
        source: io::Error::from(io::ErrorKind::InvalidInput),
        operation: "CString::new",
    })?;

    let path = path.as_ref();
    let r =
        selinux::SecurityContext::from_c_str(&c_context, false).set_for_path(path, false, false);
    if let Err(r) = &r {
        println!(
            "set_file_context failed: '{}' => '{}': {}.",
            context,
            path.display(),
            r
        );
    } else {
        println!("set_file_context: '{}' => '{}'.", context, path.display());
    }
    r
}
