// This file is part of the uutils coreutils package.
//
// (c) Sunrin SHIMURA
// Collaborator: Jian Zeng
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (paths) GPGHome

#[macro_use]
extern crate uucore;

use std::env;
use std::iter;
use std::path::{is_separator, PathBuf};

use rand::Rng;
use tempfile::Builder;

use crate::app::*;

pub mod app;

fn get_usage() -> String {
    format!("{0} [OPTION]... [TEMPLATE]", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = get_app(executable!())
        .usage(&usage[..])
        .get_matches_from(args);

    let template = matches.value_of(ARG_TEMPLATE).unwrap();
    let tmpdir = matches.value_of(OPT_TMPDIR).unwrap_or_default();

    let (template, mut tmpdir) = if matches.is_present(OPT_TMPDIR)
        && !PathBuf::from(tmpdir).is_dir() // if a temp dir is provided, it must be an actual path
        && tmpdir.contains("XXX")
    // If this is a template, it has to contain at least 3 X
        && template == DEFAULT_TEMPLATE
    // That means that clap does not think we provided a template
    {
        // Special case to workaround a limitation of clap when doing
        // mktemp --tmpdir apt-key-gpghome.XXX
        // The behavior should be
        // mktemp --tmpdir $TMPDIR apt-key-gpghome.XX
        // As --tmpdir is empty
        //
        // Fixed in clap 3
        // See https://github.com/clap-rs/clap/pull/1587
        let tmp = env::temp_dir();
        (tmpdir, tmp)
    } else if !matches.is_present(OPT_TMPDIR) {
        let tmp = env::temp_dir();
        (template, tmp)
    } else {
        (template, PathBuf::from(tmpdir))
    };

    let make_dir = matches.is_present(OPT_DIRECTORY);
    let dry_run = matches.is_present(OPT_DRY_RUN);
    let suppress_file_err = matches.is_present(OPT_QUIET);

    let (prefix, rand, suffix) = match parse_template(template) {
        Some((p, r, s)) => match matches.value_of(OPT_SUFFIX) {
            Some(suf) => {
                if s.is_empty() {
                    (p, r, suf)
                } else {
                    crash!(
                        1,
                        "Template should end with 'X' when you specify suffix option."
                    )
                }
            }
            None => (p, r, s),
        },
        None => ("", 0, ""),
    };

    if rand < 3 {
        crash!(1, "Too few 'X's in template")
    }

    if suffix.chars().any(is_separator) {
        crash!(1, "suffix cannot contain any path separators");
    }

    if matches.is_present(OPT_TMPDIR) && PathBuf::from(prefix).is_absolute() {
        show_error!(
            "invalid template, ‘{}’; with --tmpdir, it may not be absolute",
            template
        );
        return 1;
    };

    if matches.is_present(OPT_T) {
        tmpdir = env::temp_dir()
    };

    if dry_run {
        dry_exec(tmpdir, prefix, rand, suffix)
    } else {
        exec(tmpdir, prefix, rand, suffix, make_dir, suppress_file_err)
    }
}

fn parse_template(temp: &str) -> Option<(&str, usize, &str)> {
    let right = match temp.rfind('X') {
        Some(r) => r + 1,
        None => return None,
    };
    let left = temp[..right].rfind(|c| c != 'X').map_or(0, |i| i + 1);
    let prefix = &temp[..left];
    let rand = right - left;
    let suffix = &temp[right..];
    Some((prefix, rand, suffix))
}

pub fn dry_exec(mut tmpdir: PathBuf, prefix: &str, rand: usize, suffix: &str) -> i32 {
    let len = prefix.len() + suffix.len() + rand;
    let mut buf = String::with_capacity(len);
    buf.push_str(prefix);
    buf.extend(iter::repeat('X').take(rand));
    buf.push_str(suffix);

    // Randomize.
    unsafe {
        // We guarantee utf8.
        let bytes = &mut buf.as_mut_vec()[prefix.len()..prefix.len() + rand];
        rand::thread_rng().fill(bytes);
        for byte in bytes.iter_mut() {
            *byte = match *byte % 62 {
                v @ 0..=9 => (v + b'0'),
                v @ 10..=35 => (v - 10 + b'a'),
                v @ 36..=61 => (v - 36 + b'A'),
                _ => unreachable!(),
            }
        }
    }
    tmpdir.push(buf);
    println!("{}", tmpdir.display());
    0
}

fn exec(dir: PathBuf, prefix: &str, rand: usize, suffix: &str, make_dir: bool, quiet: bool) -> i32 {
    let res = if make_dir {
        let tmpdir = Builder::new()
            .prefix(prefix)
            .rand_bytes(rand)
            .suffix(suffix)
            .tempdir_in(&dir);

        // `into_path` consumes the TempDir without removing it
        tmpdir.map(|d| d.into_path().to_string_lossy().to_string())
    } else {
        let tmpfile = Builder::new()
            .prefix(prefix)
            .rand_bytes(rand)
            .suffix(suffix)
            .tempfile_in(&dir);

        match tmpfile {
            Ok(f) => {
                // `keep` ensures that the file is not deleted
                match f.keep() {
                    Ok((_, p)) => Ok(p.to_string_lossy().to_string()),
                    Err(e) => {
                        show_error!("'{}': {}", dir.display(), e);
                        return 1;
                    }
                }
            }
            Err(x) => Err(x),
        }
    };

    match res {
        Ok(ref f) => {
            println!("{}", f);
            0
        }
        Err(e) => {
            if !quiet {
                show_error!("{}: {}", e, dir.display());
            }
            1
        }
    }
}
