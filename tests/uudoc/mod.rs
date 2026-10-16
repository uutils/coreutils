// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore manpages

use std::{env, path::PathBuf, process::Command, sync::OnceLock};

static UUDOC_BINARY_PATH: OnceLock<PathBuf> = OnceLock::new();

fn get_uudoc_command() -> Command {
    let uudoc_binary = UUDOC_BINARY_PATH.get_or_init(|| {
        let coreutils_binary = PathBuf::from(env!("CARGO_BIN_EXE_coreutils"));
        coreutils_binary.parent().unwrap().join("uudoc")
    });
    Command::new(uudoc_binary)
}

#[test]
fn test_manpage_generation() {
    let temp = tempfile::tempdir().unwrap();
    let output = get_uudoc_command()
        .args(["manpages", "--usr"])
        .arg(temp.path())
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed with status: {}",
        output.status
    );
    // Note: tldr warning is now printed at build time (in build.rs), not at runtime
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty but got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(output.stdout.is_empty());
    let output_str = std::fs::read_to_string(temp.path().join("share/man/man1/ls.1")).unwrap();
    assert!(output_str.contains("\n.TH LS"), "{output_str}");
    assert!(output_str.contains('1'), "{output_str}");
    assert!(output_str.contains("\n.SH NAME\nls"), "{output_str}");
}

#[test]
fn test_manpage_coreutils() {
    let temp = tempfile::tempdir().unwrap();
    let output = get_uudoc_command()
        .args(["manpages", "--usr"])
        .arg(temp.path())
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed with status: {}",
        output.status
    );
    // Note: tldr warning is now printed at build time (in build.rs), not at runtime
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty but got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(output.stdout.is_empty());
    let output_str =
        std::fs::read_to_string(temp.path().join("share/man/man1/coreutils.1")).unwrap();
    assert!(output_str.contains("\n.TH COREUTILS"), "{output_str}");
    assert!(output_str.contains("coreutils"), "{output_str}");
    assert!(output_str.contains("\n.SH NAME\ncoreutils"), "{output_str}");
}

#[test]
fn test_invalid_format() {
    let temp = tempfile::tempdir().unwrap();
    let output = get_uudoc_command()
        .args(["manpages", "nonexistent_format", "--usr"])
        .arg(temp.path())
        .output()
        .expect("Failed to execute command");

    // Should fail for an invalid format
    assert!(!output.status.success(), "Command should have failed");
}

#[test]
fn test_completion_generation() {
    let temp = tempfile::tempdir().unwrap();
    let output = get_uudoc_command()
        .args(["completions", "--usr"])
        .arg(temp.path())
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed with status: {}",
        output.status
    );
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty but got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(output.stdout.is_empty());
    let output_str = std::fs::read_to_string(temp.path().join("share/powershell/_ls.ps1")).unwrap();
    assert!(
        output_str.contains("using namespace System.Management.Automation"),
        "{output_str}"
    );
}

// Prevent regression to:
//
// ❯ uudoc manpages --usr .
// ❯ rg --fixed-strings -- 'base32' share/man/man1/base64.1
// The data are encoded as described for the base32 alphabet in RFC 4648.
// to the bytes of the formal base32 alphabet. Use \-\-ignore\-garbage
// The data are encoded as described for the base32 alphabet in RFC 4648.
// to the bytes of the formal base32 alphabet. Use \-\-ignore\-garbage
#[test]
fn test_manpage_base64() {
    let temp = tempfile::tempdir().unwrap();
    let output = get_uudoc_command()
        .args(["manpages", "--usr"])
        .arg(temp.path())
        .env("LANG", "C") // force locale to english to avoid issues with manpage output
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed with status: {}",
        output.status
    );
    // Note: tldr warning is now printed at build time (in build.rs), not at runtime
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty but got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(output.stdout.is_empty());
    let output_str = std::fs::read_to_string(temp.path().join("share/man/man1/base64.1")).unwrap();
    assert!(output_str.contains("base64 alphabet"));
    assert!(!output_str.to_ascii_lowercase().contains("base32"));
}

// Test to ensure markdown headers are correctly formatted in generated markdown files
// Prevents regression of https://github.com/uutils/coreutils/issues/10003
#[test]
fn test_markdown_header_format() {
    use std::fs;

    // Read a sample markdown file from the documentation
    // This assumes the docs have been generated (they should be in the repo)
    let docs_path = "docs/src/utils/cat.md";

    if fs::metadata(docs_path).is_ok() {
        let content =
            fs::read_to_string(docs_path).expect("Failed to read generated markdown file");

        // Verify Options header is in markdown format (## Options)
        assert!(
            content.contains("## Options"),
            "Generated markdown should contain '## Options' header"
        );

        // Verify no HTML h2 tags for Options (old format)
        assert!(
            !content.contains("<h2>Options</h2>"),
            "Generated markdown should not contain '<h2>Options</h2>' (use markdown format instead)"
        );

        // Also verify Examples if it exists
        if content.contains("## Examples") {
            assert!(
                content.contains("## Examples"),
                "Generated markdown should contain '## Examples' header in markdown format"
            );
        }
    }
}

#[test]
fn test_source_date_epoch() {
    let temp = tempfile::tempdir().unwrap();
    let output = get_uudoc_command()
        .args(["manpages", "--usr"])
        .arg(temp.path())
        .env("SOURCE_DATE_EPOCH", "123456")
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed with status: {}",
        output.status
    );
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty but got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(output.stdout.is_empty());
    let output_str = std::fs::read_to_string(temp.path().join("share/man/man1/ls.1")).unwrap();
    let first_line = output_str
        .lines()
        .find(|line| line.starts_with(".TH"))
        .unwrap();
    assert_eq!(first_line, ".TH LS 1 \"1970-01-02\"");
}

#[test]
fn test_manpage_output_directory() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("generated docs");

    for (prefix, filename) in [("", "ls.1"), ("uu-", "uu-ls.1")] {
        let output = get_uudoc_command()
            .args(["manpages", "--usr"])
            .arg(&directory)
            .env("PROG_PREFIX", prefix)
            .env("SOURCE_DATE_EPOCH", "123456")
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
        let manpage =
            std::fs::read_to_string(directory.join("share/man/man1").join(filename)).unwrap();
        assert!(manpage.contains(".TH LS 1 \"1970-01-02\""), "{manpage}");
        assert!(manpage.contains("\n.SH NAME\nls"), "{manpage}");
    }
}

#[test]
fn test_completion_output_directory() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("shell completions");

    let output = get_uudoc_command()
        .args(["completions", "--usr"])
        .arg(&directory)
        .env("PROG_PREFIX", "uu-")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    for filename in [
        "bash-completion/completions/uu-ls.bash",
        "elvish/lib/uu-ls.elv",
        "fish/vendor_completions.d/uu-ls.fish",
        "powershell/_uu-ls.ps1",
        "zsh/site-functions/_uu-ls",
    ] {
        let completion = std::fs::read_to_string(directory.join("share").join(filename)).unwrap();
        assert!(completion.contains("uu-ls"), "{completion}");
    }
}

#[rstest::rstest]
fn test_bulk_generation(#[values("", "uu-")] prefix: &str) {
    use std::collections::BTreeSet;
    use std::fs;

    let temp = tempfile::tempdir().unwrap();
    let output = get_uudoc_command()
        .args(["manpages", "md", "completions", "--usr"])
        .arg(temp.path())
        .env("PROG_PREFIX", prefix)
        .env("SOURCE_DATE_EPOCH", "123456")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());

    let list = Command::new(env!("CARGO_BIN_EXE_coreutils"))
        .arg("--list")
        .output()
        .unwrap();
    assert!(list.status.success());
    let list = String::from_utf8(list.stdout).unwrap();
    let utilities: Vec<_> = list.lines().chain(["coreutils"]).collect();

    for (directory, leading, extension) in [
        ("man/man1", "", ".1"),
        ("docs/coreutils", "", ".md"),
        ("bash-completion/completions", "", ".bash"),
        ("elvish/lib", "", ".elv"),
        ("fish/vendor_completions.d", "", ".fish"),
        ("powershell", "_", ".ps1"),
        ("zsh/site-functions", "_", ""),
    ] {
        let expected: BTreeSet<_> = utilities
            .iter()
            .filter(|&&utility| directory != "zsh/site-functions" || utility != "[")
            .map(|utility| format!("{leading}{prefix}{utility}{extension}"))
            .collect();
        let bulk_path = temp.path().join("share").join(directory);
        let actual: BTreeSet<_> = fs::read_dir(&bulk_path)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(actual, expected, "{directory}");
    }
}

#[test]
fn test_output_directory_required() {
    for args in [&["manpages", "completions"][..], &["md"][..]] {
        let output = get_uudoc_command().args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("--usr <DIR>"));
    }
}

#[test]
fn test_output_directory_errors() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("output");
    std::fs::write(&directory, "keep this file").unwrap();

    let output = get_uudoc_command()
        .args(["manpages", "completions", "--usr"])
        .arg(&directory)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    assert_eq!(
        std::fs::read_to_string(&directory).unwrap(),
        "keep this file"
    );
}

#[test]
fn test_output_file_errors() {
    for filename in [
        "man/man1/ls.1",
        "docs/coreutils/ls.md",
        "bash-completion/completions/ls.bash",
    ] {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("share").join(filename)).unwrap();
        let output = get_uudoc_command()
            .args(["manpages", "md", "completions", "--usr"])
            .arg(temp.path())
            .env_remove("PROG_PREFIX")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        assert!(temp.path().join("share").join(filename).is_dir());
    }
}

#[cfg(unix)]
#[test]
fn test_output_directory_non_utf8() {
    use std::os::unix::ffi::OsStrExt;

    let temp = tempfile::tempdir().unwrap();
    let directory = temp
        .path()
        .join(std::ffi::OsStr::from_bytes(b"output_\xff"));
    let output = get_uudoc_command()
        .args(["manpages", "completions", "--usr"])
        .arg(&directory)
        .env_remove("PROG_PREFIX")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    assert!(
        std::fs::read_to_string(directory.join("share/man/man1/ls.1"))
            .unwrap()
            .contains(".TH LS")
    );
}

#[rstest::rstest]
#[case(&["manpages"], &["man"])]
#[case(&["md"], &["docs"])]
#[case(&["completions"], &["bash-completion", "elvish", "fish", "powershell", "zsh"])]
#[case(&["manpages", "md"], &["man", "docs"])]
#[case(&["manpages", "completions"], &["man", "bash-completion", "elvish", "fish", "powershell", "zsh"])]
#[case(&["md", "completions"], &["docs", "bash-completion", "elvish", "fish", "powershell", "zsh"])]
#[case(&["manpages", "md", "completions"], &["man", "docs", "bash-completion", "elvish", "fish", "powershell", "zsh"])]
fn test_selected_formats(#[case] formats: &[&str], #[case] directories: &[&str]) {
    use std::collections::BTreeSet;

    let temp = tempfile::tempdir().unwrap();
    let prefix = temp.path().join("install prefix");
    let output = get_uudoc_command()
        .args(formats)
        .arg("--usr")
        .arg(&prefix)
        .env_remove("PROG_PREFIX")
        .env("LC_ALL", "C")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    let share = prefix.join("share");
    let actual: BTreeSet<_> = std::fs::read_dir(&share)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    let expected: BTreeSet<_> = directories.iter().map(|name| (*name).to_string()).collect();
    assert_eq!(actual, expected);
    if formats.contains(&"md") {
        let markdown = std::fs::read_to_string(share.join("docs/coreutils/ls.md")).unwrap();
        assert!(markdown.starts_with("# ls\n"), "{markdown}");
        assert!(markdown.contains("List directory contents"), "{markdown}");
        assert!(markdown.contains("## Options"), "{markdown}");
        assert!(markdown.contains("--all"), "{markdown}");
        assert!(!markdown.contains("ls-help-"), "{markdown}");
    }
}

#[test]
fn test_markdown_prefix_and_multicall() {
    let temp = tempfile::tempdir().unwrap();
    let output = get_uudoc_command()
        .args(["md", "--usr"])
        .arg(temp.path())
        .env("PROG_PREFIX", "uu-")
        .env("LC_ALL", "C")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    let directory = temp.path().join("share/docs/coreutils");
    let multicall = std::fs::read_to_string(directory.join("uu-coreutils.md")).unwrap();
    assert!(multicall.starts_with("# coreutils\n"), "{multicall}");
    let base64 = std::fs::read_to_string(directory.join("uu-base64.md")).unwrap();
    assert!(base64.contains("base64 alphabet"), "{base64}");
    assert!(!base64.to_ascii_lowercase().contains("base32"), "{base64}");
}
