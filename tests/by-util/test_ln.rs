// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
#![allow(clippy::similar_names)]

use std::path::PathBuf;
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_symlink_existing_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_symlink_existing_file";
    let link = "test_symlink_existing_file_link";

    at.touch(file);

    ucmd.args(&["-s", file, link]).succeeds().no_stderr();

    assert!(at.file_exists(file));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);
}

#[test]
fn test_symlink_dangling_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_symlink_dangling_file";
    let link = "test_symlink_dangling_file_link";

    ucmd.args(&["-s", file, link]).succeeds().no_stderr();
    assert!(!at.file_exists(file));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);
}

#[test]
fn test_symlink_existing_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_symlink_existing_dir";
    let link = "test_symlink_existing_dir_link";

    at.mkdir(dir);

    ucmd.args(&["-s", dir, link]).succeeds().no_stderr();
    assert!(at.dir_exists(dir));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), dir);
}

#[test]
fn test_symlink_dangling_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_symlink_dangling_dir";
    let link = "test_symlink_dangling_dir_link";

    ucmd.args(&["-s", dir, link]).succeeds().no_stderr();
    assert!(!at.dir_exists(dir));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), dir);
}

#[test]
fn test_symlink_circular() {
    let (at, mut ucmd) = at_and_ucmd!();
    let link = "test_symlink_circular";

    ucmd.args(&["-s", link]).succeeds().no_stderr();
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), link);
}

#[test]
fn test_symlink_do_not_overwrite() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_symlink_do_not_overwrite";
    let link = "test_symlink_do_not_overwrite_link";

    at.touch(file);
    at.touch(link);

    ucmd.args(&["-s", file, link]).fails();
    assert!(at.file_exists(file));
    assert!(at.file_exists(link));
    assert!(!at.is_symlink(link));
}

#[test]
fn test_symlink_overwrite_force() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_symlink_overwrite_force_a";
    let file_b = "test_symlink_overwrite_force_b";
    let link = "test_symlink_overwrite_force_link";

    // Create symlink
    at.symlink_file(file_a, link);
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file_a);

    // Force overwrite of existing symlink
    ucmd.args(&["--force", "-s", file_b, link]).succeeds();
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file_b);
}

#[test]
fn test_symlink_overwrite_force_overrides_interactive() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_symlink_overwrite_force_a";
    let file_b = "test_symlink_overwrite_force_b";
    let link = "test_symlink_overwrite_force_link";

    // Create symlink
    at.symlink_file(file_a, link);
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file_a);

    // Force overwrite of existing symlink
    ucmd.args(&["-i", "-f", "-s", file_b, link]).succeeds();
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file_b);
}

#[test]
fn test_symlink_interactive() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let file = "test_symlink_interactive_file";
    let link = "test_symlink_interactive_file_link";

    at.touch(file);
    at.touch(link);

    scene
        .ucmd()
        .args(&["-i", "-s", file, link])
        .pipe_in("n")
        .fails()
        .no_stdout();

    assert!(at.file_exists(file));
    assert!(!at.is_symlink(link));

    scene
        .ucmd()
        .args(&["-i", "-s", file, link])
        .pipe_in("Yesh") // spell-checker:disable-line
        .succeeds()
        .no_stdout();

    assert!(at.file_exists(file));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);
}

#[test]
fn test_symlink_interactive_overrides_force() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let file = "test_symlink_interactive_file";
    let link = "test_symlink_interactive_file_link";

    at.touch(file);
    at.touch(link);

    scene
        .ucmd()
        .args(&["-f", "-i", "-s", file, link])
        .pipe_in("n")
        .fails()
        .no_stdout();

    assert!(at.file_exists(file));
    assert!(!at.is_symlink(link));

    scene
        .ucmd()
        .args(&["-f", "-i", "-s", file, link])
        .pipe_in("Yesh") // spell-checker:disable-line
        .succeeds()
        .no_stdout();

    assert!(at.file_exists(file));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);
}

#[test]
fn test_symlink_simple_backup() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_symlink_simple_backup";
    let link = "test_symlink_simple_backup_link";

    at.touch(file);
    at.symlink_file(file, link);
    assert!(at.file_exists(file));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);

    ucmd.args(&["-b", "-s", file, link]).succeeds().no_stderr();

    assert!(at.file_exists(file));

    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);

    let backup = &format!("{link}~");
    assert!(at.is_symlink(backup));
    assert_eq!(at.resolve_link(backup), file);
}

#[test]
fn test_symlink_custom_backup_suffix() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_symlink_custom_backup_suffix";
    let link = "test_symlink_custom_backup_suffix_link";
    let suffix = "super-suffix-of-the-century";

    at.touch(file);
    at.symlink_file(file, link);
    assert!(at.file_exists(file));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);

    let arg = &format!("--suffix={suffix}");
    ucmd.args(&["-b", arg, "-s", file, link])
        .succeeds()
        .no_stderr();
    assert!(at.file_exists(file));

    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);

    let backup = &format!("{link}{suffix}");
    assert!(at.is_symlink(backup));
    assert_eq!(at.resolve_link(backup), file);
}

#[test]
fn test_symlink_suffix_without_backup_option() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("a", "a\n");
    at.write("b", "b2\n");

    assert!(at.file_exists("a"));
    assert!(at.file_exists("b"));
    let suffix = ".sfx";
    let suffix_arg = &format!("--suffix={suffix}");
    scene
        .ucmd()
        .args(&["-s", "-f", suffix_arg, "a", "b"])
        .succeeds()
        .no_stderr();
    assert!(at.file_exists("a"));
    assert!(at.file_exists("b"));
    assert_eq!(at.read("a"), "a\n");
    assert_eq!(at.read("b"), "a\n");
    // we should have created backup for b file
    assert_eq!(at.read(&format!("b{suffix}")), "b2\n");
}

#[test]
fn test_symlink_custom_backup_suffix_hyphen_value() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_symlink_custom_backup_suffix";
    let link = "test_symlink_custom_backup_suffix_link";
    let suffix = "-v";

    at.touch(file);
    at.symlink_file(file, link);
    assert!(at.file_exists(file));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);

    let arg = &format!("--suffix={suffix}");
    ucmd.args(&["-b", arg, "-s", file, link])
        .succeeds()
        .no_stderr();
    assert!(at.file_exists(file));

    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);

    let backup = &format!("{link}{suffix}");
    assert!(at.is_symlink(backup));
    assert_eq!(at.resolve_link(backup), file);
}

#[test]
fn test_symlink_backup_numbering() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_symlink_backup_numbering";
    let link = "test_symlink_backup_numbering_link";

    at.touch(file);
    at.symlink_file(file, link);
    assert!(at.file_exists(file));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);

    ucmd.args(&["-s", "--backup=t", file, link])
        .succeeds()
        .no_stderr();
    assert!(at.file_exists(file));

    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);

    let backup = &format!("{link}.~1~");
    assert!(at.is_symlink(backup));
    assert_eq!(at.resolve_link(backup), file);
}

#[test]
fn test_symlink_existing_backup() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_symlink_existing_backup";
    let link = "test_symlink_existing_backup_link";
    let link_backup = "test_symlink_existing_backup_link.~1~";
    let resulting_backup = "test_symlink_existing_backup_link.~2~";

    // Create symlink and verify
    at.touch(file);
    at.symlink_file(file, link);
    assert!(at.file_exists(file));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file);

    // Create backup symlink and verify
    at.symlink_file(file, link_backup);
    assert!(at.file_exists(file));
    assert!(at.is_symlink(link_backup));
    assert_eq!(at.resolve_link(link_backup), file);

    ucmd.args(&["-s", "--backup=nil", file, link])
        .succeeds()
        .no_stderr();
    assert!(at.file_exists(file));

    assert!(at.is_symlink(link_backup));
    assert_eq!(at.resolve_link(link_backup), file);

    assert!(at.is_symlink(resulting_backup));
    assert_eq!(at.resolve_link(resulting_backup), file);
}

#[test]
fn test_symlink_target_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_ln_target_dir_dir";
    let file_a = "test_ln_target_dir_file_a";
    let file_b = "test_ln_target_dir_file_b";

    at.touch(file_a);
    at.touch(file_b);
    at.mkdir(dir);

    ucmd.args(&["-s", "-t", dir, file_a, file_b])
        .succeeds()
        .no_stderr();

    let file_a_link = &format!("{dir}/{file_a}");
    assert!(at.is_symlink(file_a_link));
    assert_eq!(at.resolve_link(file_a_link), file_a);

    let file_b_link = &format!("{dir}/{file_b}");
    assert!(at.is_symlink(file_b_link));
    assert_eq!(at.resolve_link(file_b_link), file_b);
}

#[test]
fn test_symlink_target_dir_from_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_ln_target_dir_dir";
    let from_dir = "test_ln_target_dir_from_dir";
    let filename_a = "test_ln_target_dir_file_a";
    let filename_b = "test_ln_target_dir_file_b";
    let file_a = &format!("{from_dir}/{filename_a}");
    let file_b = &format!("{from_dir}/{filename_b}");

    at.mkdir(from_dir);
    at.touch(file_a);
    at.touch(file_b);
    at.mkdir(dir);

    ucmd.args(&["-s", "-t", dir, file_a, file_b])
        .succeeds()
        .no_stderr();

    let file_a_link = &format!("{dir}/{filename_a}");
    assert!(at.is_symlink(file_a_link));
    assert_eq!(&at.resolve_link(file_a_link), file_a);

    let file_b_link = &format!("{dir}/{filename_b}");
    assert!(at.is_symlink(file_b_link));
    assert_eq!(&at.resolve_link(file_b_link), file_b);
}

#[test]
fn test_symlink_overwrite_dir_fail() {
    let (at, mut ucmd) = at_and_ucmd!();
    let path_a = "test_symlink_overwrite_dir_a";
    let path_b = "test_symlink_overwrite_dir_b";

    at.touch(path_a);
    at.mkdir(path_b);

    assert!(
        !ucmd
            .args(&["-s", "-T", path_a, path_b])
            .fails()
            .stderr_str()
            .is_empty()
    );
}

#[test]
fn test_symlink_errors() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_symlink_errors_dir";
    let file_a = "test_symlink_errors_file_a";
    let file_b = "test_symlink_errors_file_b";

    at.mkdir(dir);
    at.touch(file_a);
    at.touch(file_b);

    // $ ln -T -t a b
    // ln: cannot combine --target-directory (-t) and --no-target-directory (-T)
    ucmd.args(&["-T", "-t", dir, file_a, file_b]).fails();
}

#[test]
fn test_symlink_verbose() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let file_a = "test_symlink_verbose_file_a";
    let file_b = "test_symlink_verbose_file_b";

    at.touch(file_a);

    scene
        .ucmd()
        .args(&["-s", "-v", file_a, file_b])
        .succeeds()
        .stdout_only(format!("'{file_b}' -> '{file_a}'\n"));

    at.touch(file_b);

    scene
        .ucmd()
        .args(&["-s", "-v", "-b", file_a, file_b])
        .succeeds()
        .stdout_only(format!("'{file_b}' -> '{file_a}' (backup: '{file_b}~')\n"));
}

#[test]
fn test_symlink_target_only() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_symlink_target_only";

    at.mkdir(dir);

    assert!(
        !ucmd
            .args(&["-s", "-t", dir])
            .fails()
            .stderr_str()
            .is_empty()
    );
}

#[test]
fn test_symlink_implicit_target_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_symlink_implicit_target_dir";
    // On windows, slashes aren't allowed in symlink targets, so use
    // PathBuf to construct `file` instead of simple "dir/file".
    let filename = "test_symlink_implicit_target_file";
    let path = PathBuf::from(dir).join(filename);
    let file = &path.to_string_lossy();

    at.mkdir(dir);
    at.touch(&path);

    ucmd.args(&["-s", file]).succeeds().no_stderr();

    assert!(at.file_exists(filename));
    assert!(at.is_symlink(filename));
    assert_eq!(at.resolve_link(filename), *file);
}

#[test]
fn test_symlink_to_dir_2args() {
    let (at, mut ucmd) = at_and_ucmd!();
    let filename = "test_symlink_to_dir_2args_file";
    let from_file = &format!("{}/{filename}", at.as_string());
    let to_dir = "test_symlink_to_dir_2args_to_dir";
    let to_file = &format!("{to_dir}/{filename}");

    at.mkdir(to_dir);
    at.touch(from_file);

    ucmd.args(&["-s", from_file, to_dir]).succeeds().no_stderr();

    assert!(at.file_exists(to_file));
    assert!(at.is_symlink(to_file));
    assert_eq!(at.resolve_link(to_file), filename);
}

#[test]
fn test_symlink_missing_destination() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_symlink_missing_destination";

    at.touch(file);

    ucmd.args(&["-s", "-T", file]).fails().stderr_is(format!(
        "ln: missing destination file operand after '{file}'\n"
    ));
}

#[test]
fn test_symlink_relative() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "test_symlink_relative_a";
    let link = "test_symlink_relative_link";

    at.touch(file_a);

    // relative symlink
    ucmd.args(&["-r", "-s", file_a, link]).succeeds();
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file_a);
}

#[test]
fn test_symlink_relative_path() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "test_symlink_existing_dir";
    let file_a = "test_symlink_relative_a";
    let link = "test_symlink_relative_link";
    let multi_dir =
        "test_symlink_existing_dir/../test_symlink_existing_dir/../test_symlink_existing_dir/../";
    let p = PathBuf::from(multi_dir).join(file_a);
    at.mkdir(dir);

    // relative symlink
    // Thanks to -r, all the ../ should be resolved to a single file
    ucmd.args(&["-r", "-s", "-v", &p.to_string_lossy(), link])
        .succeeds()
        .stdout_only(format!("'{link}' -> '{file_a}'\n"));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file_a);

    // Run the same command without -r to verify that we keep the full
    // crazy path
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-s", "-v", &p.to_string_lossy(), link])
        .succeeds()
        .stdout_only(format!("'{link}' -> '{}'\n", p.to_string_lossy()));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), p.to_string_lossy());
}

#[test]
fn test_symlink_relative_dir() {
    let (at, mut ucmd) = at_and_ucmd!();

    let dir = "test_symlink_existing_dir";
    let link = "test_symlink_existing_dir_link";

    at.mkdir(dir);

    ucmd.args(&["-s", "-r", dir, link]).succeeds().no_stderr();
    assert!(at.dir_exists(dir));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), dir);
}

#[test]
fn test_symlink_no_deref_dir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let dir1 = "foo";
    let dir2 = "bar";
    let link = "baz";

    at.mkdir(dir1);
    at.mkdir(dir2);
    scene
        .ucmd()
        .args(&["-s", dir2, link])
        .succeeds()
        .no_stderr();
    assert!(at.dir_exists(dir1));
    assert!(at.dir_exists(dir2));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), dir2);

    // try the normal behavior
    scene
        .ucmd()
        .args(&["-sf", dir1, link])
        .succeeds()
        .no_stderr();
    assert!(at.dir_exists(dir1));
    assert!(at.dir_exists(dir2));
    assert!(at.is_symlink("baz/foo"));
    assert_eq!(at.resolve_link("baz/foo"), dir1);

    // Doesn't work without the force
    scene.ucmd().args(&["-sn", dir1, link]).fails();

    // Try with the no-deref
    scene
        .ucmd()
        .args(&["-sfn", dir1, link])
        .succeeds()
        .no_stderr();
    assert!(at.dir_exists(dir1));
    assert!(at.dir_exists(dir2));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), dir1);
}

#[test]
fn test_symlink_no_deref_file_in_destination_dir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file1 = "foo";
    let file2 = "bar";

    let dest = "baz";

    let link1 = "baz/foo";
    let link2 = "baz/bar";

    at.touch(file1);
    at.touch(file2);
    at.mkdir(dest);

    assert!(at.file_exists(file1));
    assert!(at.file_exists(file2));
    assert!(at.dir_exists(dest));

    // -n and -f should work alone
    scene
        .ucmd()
        .args(&["-sn", file1, dest])
        .succeeds()
        .no_stderr();
    assert!(at.is_symlink(link1));
    assert_eq!(at.resolve_link(link1), file1);

    scene
        .ucmd()
        .args(&["-sf", file1, dest])
        .succeeds()
        .no_stderr();
    assert!(at.is_symlink(link1));
    assert_eq!(at.resolve_link(link1), file1);

    // -n alone should fail if destination exists already (it should now)
    scene.ucmd().args(&["-sn", file1, dest]).fails();

    // -nf should also work
    scene
        .ucmd()
        .args(&["-snf", file1, dest])
        .succeeds()
        .no_stderr();
    assert!(at.is_symlink(link1));
    assert_eq!(at.resolve_link(link1), file1);

    scene
        .ucmd()
        .args(&["-snf", file1, file2, dest])
        .succeeds()
        .no_stderr();
    assert!(at.is_symlink(link1));
    assert_eq!(at.resolve_link(link1), file1);
    assert!(at.is_symlink(link2));
    assert_eq!(at.resolve_link(link2), file2);
}

#[test]
fn test_symlink_no_deref_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file1 = "foo";
    let file2 = "bar";
    let link = "baz";

    at.touch(file1);
    at.touch(file2);
    scene
        .ucmd()
        .args(&["-s", file2, link])
        .succeeds()
        .no_stderr();
    assert!(at.file_exists(file1));
    assert!(at.file_exists(file2));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file2);

    // try the normal behavior
    scene
        .ucmd()
        .args(&["-sf", file1, link])
        .succeeds()
        .no_stderr();
    assert!(at.file_exists(file1));
    assert!(at.file_exists(file2));
    assert!(at.is_symlink("baz"));
    assert_eq!(at.resolve_link("baz"), file1);

    // Doesn't work without the force
    scene.ucmd().args(&["-sn", file1, link]).fails();

    // Try with the no-deref
    scene.ucmd().args(&["-sfn", file1, link]).succeeds();
    assert!(at.file_exists(file1));
    assert!(at.file_exists(file2));
    assert!(at.is_symlink(link));
    assert_eq!(at.resolve_link(link), file1);
}

#[test]
fn test_relative_requires_symbolic() {
    new_ucmd!().args(&["-r", "foo", "bar"]).fails();
}

#[test]
fn test_relative_dst_already_symlink() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file1");
    at.symlink_file("file1", "file2");
    ucmd.arg("-srf").arg("file1").arg("file2").succeeds();
    at.is_symlink("file2");
}

#[test]
fn test_relative_src_already_symlink() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file1");
    at.symlink_file("file1", "file2");
    ucmd.arg("-sr").arg("file2").arg("file3").succeeds();
    assert!(at.resolve_link("file3").ends_with("file1"));
}

#[test]
fn test_relative_recursive() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir");
    ucmd.args(&["-sr", "dir", "dir/recursive"]).succeeds();
    assert_eq!(at.resolve_link("dir/recursive"), ".");
}

#[test]
fn test_backup_same_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file1");
    ucmd.args(&["--backup", "file1", "./file1"])
        .fails()
        .stderr_contains("n: 'file1' and './file1' are the same file");
}

#[test]
fn test_backup_force() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("a", "a\n");
    at.write("b", "b2\n");

    scene.ucmd().args(&["-s", "b", "b~"]).succeeds().no_stderr();
    assert!(at.file_exists("a"));
    assert!(at.file_exists("b"));
    assert!(at.file_exists("b~"));
    scene
        .ucmd()
        .args(&["-s", "-f", "--b=simple", "a", "b"])
        .succeeds()
        .no_stderr();
    assert!(at.file_exists("a"));
    assert!(at.file_exists("b"));
    assert!(at.file_exists("b~"));
    assert_eq!(at.read("a"), "a\n");
    assert_eq!(at.read("b"), "a\n");
    // we should have the same content as b as we had time to do a backup
    assert_eq!(at.read("b~"), "b2\n");
}

#[test]
fn test_hard_logical() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file_a = "file1";
    let link = "symlink1";
    let target = "hard-to-a";
    let target2 = "hard-to-a2";
    at.touch(file_a);
    at.symlink_file(file_a, link);

    ucmd.args(&["-P", "-L", link, target]);
    assert!(!at.is_symlink(target));

    ucmd.args(&["-P", "-L", "-s", link, target2]);
    assert!(!at.is_symlink(target2));
}

#[test]
fn test_hard_logical_non_exit_fail() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file_a = "/no-such-dir";
    let link = "hard-to-dangle";

    at.relative_symlink_dir(file_a, "no-such-dir");

    scene
        .ucmd()
        .args(&["-L", "no-such-dir", link])
        .fails()
        .stderr_contains("failed to access 'no-such-dir'");
}

#[test]
fn test_hard_logical_dir_fail() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let dir = "d";
    at.mkdir(dir);
    let target = "link-to-dir";

    scene.ucmd().args(&["-s", dir, target]);

    scene
        .ucmd()
        .args(&["-L", target, "hard-to-dir-link"])
        .fails()
        .stderr_contains("failed to create hard link 'link-to-dir'");
}

#[test]
fn test_symlink_remove_existing_same_src_and_dest() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("a");
    at.write("a", "sample");
    ucmd.args(&["-sf", "a", "a"])
        .fails_with_code(1)
        .stderr_contains("'a' and 'a' are the same file");
    assert!(at.file_exists("a") && !at.symlink_exists("a"));
    assert_eq!(at.read("a"), "sample");
}

#[test]
fn test_force_same_file_detected_after_canonicalization() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.write("file", "hello");

    ucmd.args(&["-f", "file", "./file"])
        .fails_with_code(1)
        .stderr_contains("are the same file");

    assert!(at.file_exists("file"));
    assert_eq!(at.read("file"), "hello");
}

#[test]
#[cfg(not(target_os = "android"))]
fn test_force_ln_existing_hard_link_entry() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("file", "hardlink\n");
    at.mkdir("dir");

    scene.ucmd().args(&["file", "dir"]).succeeds().no_stderr();
    assert!(at.file_exists("dir/file"));

    scene
        .ucmd()
        .args(&["-f", "file", "dir"])
        .succeeds()
        .no_stderr();

    assert!(at.file_exists("file"));
    assert!(at.file_exists("dir/file"));
    assert_eq!(at.read("file"), "hardlink\n");
    assert_eq!(at.read("dir/file"), "hardlink\n");

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let source_inode = at.metadata("file").ino();
        let target_inode = at.metadata("dir/file").ino();
        assert_eq!(source_inode, target_inode);
    }
}

#[test]
#[cfg(not(target_os = "android"))]
fn test_ln_seen_file() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    at.mkdir("a");
    at.mkdir("b");
    at.mkdir("c");
    at.write("a/f", "a");
    at.write("b/f", "b");

    let result = ts.ucmd().arg("a/f").arg("b/f").arg("c").fails();

    #[cfg(not(unix))]
    assert!(
        result
            .stderr_str()
            .contains("will not overwrite just-created 'c\\f' with 'b/f'")
    );
    #[cfg(unix)]
    assert!(
        result
            .stderr_str()
            .contains("will not overwrite just-created 'c/f' with 'b/f'")
    );

    assert!(at.plus("c").join("f").exists());
    // b/f still exists
    assert!(at.plus("b").join("f").exists());
    // a/f still exists
    assert!(at.plus("a").join("f").exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // Check inode numbers
        let inode_a_f = at.plus("a").join("f").metadata().unwrap().ino();
        let inode_b_f = at.plus("b").join("f").metadata().unwrap().ino();
        let inode_c_f = at.plus("c").join("f").metadata().unwrap().ino();

        assert_eq!(
            inode_a_f, inode_c_f,
            "Inode numbers of a/f and c/f should be equal"
        );
        assert_ne!(
            inode_b_f, inode_c_f,
            "Inode numbers of b/f and c/f should not be equal"
        );
    }
}

#[test]
#[cfg(target_os = "linux")]
fn test_ln_non_utf8_paths() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Create a test file with non-UTF-8 bytes in the name
    let non_utf8_bytes = b"test_\xFF\xFE.txt";
    let non_utf8_name = OsStr::from_bytes(non_utf8_bytes);
    let non_utf8_link_bytes = b"link_\xFF\xFE.txt";
    let non_utf8_link_name = OsStr::from_bytes(non_utf8_link_bytes);

    // Create the actual file
    at.touch(non_utf8_name);

    // Test creating a hard link with non-UTF-8 file names
    scene
        .ucmd()
        .arg(non_utf8_name)
        .arg(non_utf8_link_name)
        .succeeds();

    // Both files should exist
    assert!(at.file_exists(non_utf8_name));
    assert!(at.file_exists(non_utf8_link_name));

    // Test creating a symbolic link with non-UTF-8 file names
    let symlink_bytes = b"symlink_\xFF\xFE.txt";
    let symlink_name = OsStr::from_bytes(symlink_bytes);

    scene
        .ucmd()
        .args(&["-s"])
        .arg(non_utf8_name)
        .arg(symlink_name)
        .succeeds();

    // Check if symlink was created successfully
    let symlink_path = at.plus(symlink_name);
    assert!(symlink_path.is_symlink());
}

#[test]
fn test_ln_hard_link_dir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir("dir");

    scene
        .ucmd()
        .args(&["dir", "dir_link"])
        .fails()
        .stderr_contains("hard link not allowed for directory");
}
