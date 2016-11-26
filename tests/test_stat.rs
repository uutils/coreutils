use common::util::*;

extern crate uu_stat;
pub use self::uu_stat::*;


#[cfg(test)]
mod test_fsext {
    use super::*;

    #[test]
    fn test_access() {
        assert_eq!("drwxr-xr-x", pretty_access(S_IFDIR | 0o755));
        assert_eq!("-rw-r--r--", pretty_access(S_IFREG | 0o644));
        assert_eq!("srw-r-----", pretty_access(S_IFSOCK | 0o640));
        assert_eq!("lrw-r-xr-x", pretty_access(S_IFLNK | 0o655));
        assert_eq!("?rw-r-xr-x", pretty_access(0o655));

        assert_eq!("brwSr-xr-x",
                   pretty_access(S_IFBLK | S_ISUID as mode_t | 0o655));
        assert_eq!("brwsr-xr-x",
                   pretty_access(S_IFBLK | S_ISUID as mode_t | 0o755));

        assert_eq!("prw---sr--",
                   pretty_access(S_IFIFO | S_ISGID as mode_t | 0o614));
        assert_eq!("prw---Sr--",
                   pretty_access(S_IFIFO | S_ISGID as mode_t | 0o604));

        assert_eq!("c---r-xr-t",
                   pretty_access(S_IFCHR | S_ISVTX as mode_t | 0o055));
        assert_eq!("c---r-xr-T",
                   pretty_access(S_IFCHR | S_ISVTX as mode_t | 0o054));
    }

    #[test]
    fn test_file_type() {
        assert_eq!("block special file", pretty_filetype(S_IFBLK, 0));
        assert_eq!("character special file", pretty_filetype(S_IFCHR, 0));
        assert_eq!("regular file", pretty_filetype(S_IFREG, 1));
        assert_eq!("regular empty file", pretty_filetype(S_IFREG, 0));
        assert_eq!("weird file", pretty_filetype(0, 0));
    }

    #[test]
    fn test_fs_type() {
        assert_eq!("ext2/ext3", pretty_fstype(0xEF53));
        assert_eq!("tmpfs", pretty_fstype(0x01021994));
        assert_eq!("nfs", pretty_fstype(0x6969));
        assert_eq!("btrfs", pretty_fstype(0x9123683e));
        assert_eq!("xfs", pretty_fstype(0x58465342));
        assert_eq!("zfs", pretty_fstype(0x2FC12FC1));
        assert_eq!("ntfs", pretty_fstype(0x5346544e));
        assert_eq!("fat", pretty_fstype(0x4006));
        assert_eq!("UNKNOWN (0x1234)", pretty_fstype(0x1234));
    }
}

#[test]
fn test_scanutil() {
    assert_eq!(Some((-5, 2)), "-5zxc".scan_num::<i32>());
    assert_eq!(Some((51, 2)), "51zxc".scan_num::<u32>());
    assert_eq!(Some((192, 4)), "+192zxc".scan_num::<i32>());
    assert_eq!(None, "z192zxc".scan_num::<i32>());

    assert_eq!(Some(('a', 3)), "141zxc".scan_char(8));
    assert_eq!(Some(('\n', 2)), "12qzxc".scan_char(8));
    assert_eq!(Some(('\r', 1)), "dqzxc".scan_char(16));
    assert_eq!(None, "z2qzxc".scan_char(8));
}

#[test]
fn test_groupnum() {
    assert_eq!("12,379,821,234", group_num("12379821234"));
    assert_eq!("21,234", group_num("21234"));
    assert_eq!("821,234", group_num("821234"));
    assert_eq!("1,821,234", group_num("1821234"));
    assert_eq!("1,234", group_num("1234"));
    assert_eq!("234", group_num("234"));
    assert_eq!("24", group_num("24"));
    assert_eq!("4", group_num("4"));
    assert_eq!("", group_num(""));
}

#[cfg(test)]
mod test_generate_tokens {
    use super::*;

    #[test]
    fn normal_format() {
        let s = "%'010.2ac%-#5.w\n";
        let expected = vec![Token::Directive {
                                flag: F_GROUP | F_ZERO,
                                width: 10,
                                precision: 2,
                                format: 'a',
                            },
                            Token::Char('c'),
                            Token::Directive {
                                flag: F_LEFT | F_ALTER,
                                width: 5,
                                precision: 0,
                                format: 'w',
                            },
                            Token::Char('\n')];
        assert_eq!(&expected, &Stater::generate_tokens(s, false).unwrap());
    }

    #[test]
    fn printf_format() {
        let s = "%-# 15a\\r\\\"\\\\\\a\\b\\e\\f\\v%+020.-23w\\x12\\167\\132\\112\\n";
        let expected = vec![Token::Directive {
                                flag: F_LEFT | F_ALTER | F_SPACE,
                                width: 15,
                                precision: -1,
                                format: 'a',
                            },
                            Token::Char('\r'),
                            Token::Char('"'),
                            Token::Char('\\'),
                            Token::Char('\x07'),
                            Token::Char('\x08'),
                            Token::Char('\x1B'),
                            Token::Char('\x0C'),
                            Token::Char('\x0B'),
                            Token::Directive {
                                flag: F_SIGN | F_ZERO,
                                width: 20,
                                precision: -1,
                                format: 'w',
                            },
                            Token::Char('\x12'),
                            Token::Char('w'),
                            Token::Char('Z'),
                            Token::Char('J'),
                            Token::Char('\n')];
        assert_eq!(&expected, &Stater::generate_tokens(s, true).unwrap());
    }
}

#[test]
fn test_invalid_option() {
    new_ucmd!()
        .arg("-w").arg("-q").arg("/").fails();
}

#[cfg(target_os = "linux")]
const NORMAL_FMTSTR: &'static str = "%a %A %b %B %d %D %f %F %g %G %h %i %m %n %o %s %u %U %w %W %x %X %y %Y %z %Z";
#[cfg(target_os = "linux")]
const DEV_FMTSTR: &'static str = "%a %A %b %B %d %D %f %F %g %G %h %i %m %n %o %s (%t/%T) %u %U %w %W %x %X %y %Y %z %Z";
#[cfg(target_os = "linux")]
const FS_FMTSTR: &'static str = "%a %b %c %d %f %i %l %n %s %S %t %T";

#[test]
#[cfg(target_os = "linux")]
fn test_terse_fs_format() {
    let args = ["-f", "-t", "/proc"];
    new_ucmd!().args(&args)
        .run()
        .stdout_is(expected_result(&args));
}

#[test]
#[cfg(target_os = "linux")]
fn test_fs_format() {
    let args = ["-f", "-c", FS_FMTSTR, "/dev/shm"];
    new_ucmd!().args(&args)
        .run()
        .stdout_is(expected_result(&args));
}

#[test]
#[cfg(target_os = "linux")]
fn test_terse_normal_format() {
    let args = ["-t", "/"];
    new_ucmd!().args(&args)
        .run()
        .stdout_is(expected_result(&args));
}

#[test]
#[cfg(target_os = "linux")]
fn test_normal_format() {
    let args = ["-c", NORMAL_FMTSTR, "/boot"];
    new_ucmd!().args(&args)
        .run()
        .stdout_is(expected_result(&args));
}

#[test]
#[cfg(target_os = "linux")]
fn test_follow_symlink() {
    let args = ["-L", "-c", DEV_FMTSTR, "/dev/cdrom"];
    new_ucmd!().args(&args)
        .run()
        .stdout_is(expected_result(&args));
}

#[test]
#[cfg(target_os = "linux")]
fn test_symlink() {
    let args = ["-c", DEV_FMTSTR, "/dev/cdrom"];
    new_ucmd!().args(&args)
        .run()
        .stdout_is(expected_result(&args));
}

#[test]
#[cfg(target_os = "linux")]
fn test_char() {
    let args = ["-c", DEV_FMTSTR, "/dev/pts/ptmx"];
    new_ucmd!().args(&args)
        .run()
        .stdout_is(expected_result(&args));
}

#[test]
#[cfg(target_os = "linux")]
fn test_multi_files() {
    let args = ["-c", NORMAL_FMTSTR, "/dev", "/usr/lib", "/etc/fstab", "/var"];
    new_ucmd!().args(&args)
        .run()
        .stdout_is(expected_result(&args));
}

#[test]
#[cfg(target_os = "linux")]
fn test_printf() {
    let args = ["--printf=123%-# 15q\\r\\\"\\\\\\a\\b\\e\\f\\v%+020.23m\\x12\\167\\132\\112\\n", "/"];
    new_ucmd!().args(&args)
        .run()
        .stdout_is(expected_result(&args));
}

#[cfg(target_os = "linux")]
fn expected_result(args: &[&str]) -> String {
    use std::process::Command;

    let output = Command::new(util_name!()).args(args).output().unwrap();
    String::from_utf8_lossy(&output.stdout).into_owned()
}
