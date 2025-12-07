// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore prefixcat testcat

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process;

use uucore::Args;
use uucore::display::Quotable;
use uucore::locale;

/// Gets all available utilities including "coreutils"
#[allow(clippy::type_complexity)]
pub fn get_all_utilities<T: Args>(
    util_map: &phf::OrderedMap<&'static str, (fn(T) -> i32, fn() -> clap::Command)>,
) -> Vec<&'static str> {
    std::iter::once("coreutils")
        .chain(util_map.keys().copied())
        .collect()
}

/// Prints a "utility not found" error and exits
pub fn not_found(util: &OsStr) -> ! {
    eprintln!("{}: function/utility not found", util.maybe_quote());
    process::exit(1);
}

/// Sets up localization for a utility with proper error handling
pub fn setup_localization_or_exit(util_name: &str) {
    let util_name = get_canonical_util_name(util_name);
    locale::setup_localization(util_name).unwrap_or_else(|err| {
        match err {
            locale::LocalizationError::ParseResource {
                error: err_msg,
                snippet,
            } => eprintln!("Localization parse error at {snippet}: {err_msg}"),
            other => eprintln!("Could not init the localization system: {other}"),
        }
        process::exit(99)
    });
}

/// Gets the canonical utility name, resolving aliases
fn get_canonical_util_name(util_name: &str) -> &str {
    match util_name {
        // uu_test aliases - '[' is an alias for test
        "[" => "test",

        // hashsum aliases - all these hash commands are aliases for hashsum
        "md5sum" | "sha1sum" | "sha224sum" | "sha256sum" | "sha384sum" | "sha512sum" | "b2sum" => {
            "hashsum"
        }

        "dir" => "ls", // dir is an alias for ls

        // Default case - return the util name as is
        _ => util_name,
    }
}

/// Finds a utility with a prefix (e.g., "uu_test" -> "test")
pub fn find_prefixed_util<'a>(
    binary_name: &str,
    mut util_keys: impl Iterator<Item = &'a str>,
) -> Option<&'a str> {
    util_keys.find(|util| {
        binary_name.ends_with(*util)
            && binary_name.len() > util.len() // Ensure there's actually a prefix
            && !binary_name[..binary_name.len() - (*util).len()]
                .ends_with(char::is_alphanumeric)
    })
}

/// Gets the binary path from command line arguments
/// # Panics
/// Panics if the binary path cannot be determined
pub fn binary_path(args: &mut impl Iterator<Item = OsString>) -> PathBuf {
    match args.next() {
        Some(ref s) if !s.is_empty() => PathBuf::from(s),
        _ => std::env::current_exe().unwrap(),
    }
}

/// Extracts the binary name from a path
pub fn name(binary_path: &Path) -> Option<&str> {
    binary_path.file_stem()?.to_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_canonical_util_name() {
        // Test a few key aliases
        assert_eq!(get_canonical_util_name("["), "test");
        assert_eq!(get_canonical_util_name("md5sum"), "hashsum");
        assert_eq!(get_canonical_util_name("dir"), "ls");

        // Test passthrough case
        assert_eq!(get_canonical_util_name("cat"), "cat");
    }

    #[test]
    fn test_name() {
        // Test normal executable name
        assert_eq!(name(Path::new("/usr/bin/ls")), Some("ls"));
        assert_eq!(name(Path::new("cat")), Some("cat"));
        assert_eq!(
            name(Path::new("./target/debug/coreutils")),
            Some("coreutils")
        );

        // Test with extensions
        assert_eq!(name(Path::new("program.exe")), Some("program"));
        assert_eq!(name(Path::new("/path/to/utility.bin")), Some("utility"));

        // Test edge cases
        assert_eq!(name(Path::new("")), None);
        assert_eq!(name(Path::new("/")), None);
    }

    #[test]
    fn test_find_prefixed_util() {
        let utils = ["test", "cat", "ls", "cp"];

        // Test exact prefixed matches
        assert_eq!(
            find_prefixed_util("uu_test", utils.iter().copied()),
            Some("test")
        );
        assert_eq!(
            find_prefixed_util("my-cat", utils.iter().copied()),
            Some("cat")
        );
        assert_eq!(
            find_prefixed_util("prefix_ls", utils.iter().copied()),
            Some("ls")
        );

        // Test non-alphanumeric separator requirement
        assert_eq!(find_prefixed_util("prefixcat", utils.iter().copied()), None); // no separator
        assert_eq!(find_prefixed_util("testcat", utils.iter().copied()), None); // no separator

        // Test no match
        assert_eq!(find_prefixed_util("unknown", utils.iter().copied()), None);
        assert_eq!(find_prefixed_util("", utils.iter().copied()), None);

        // Test exact util name (should not match as prefixed)
        assert_eq!(find_prefixed_util("test", utils.iter().copied()), None);
        assert_eq!(find_prefixed_util("cat", utils.iter().copied()), None);
    }
}
