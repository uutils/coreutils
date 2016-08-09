use common::util::*;

extern crate uu_chown;
pub use self::uu_chown::*;

static UTIL_NAME: &'static str = "chown";

fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[cfg(test)]
mod test_passwd {
    use super::passwd::*;

    #[test]
    fn test_getuid() {
        assert_eq!(0, getuid("root").unwrap());
        assert!(getuid("88888888").is_err());
        assert!(getuid("auserthatdoesntexist").is_err());
    }

    #[test]
    fn test_getgid() {
        if cfg!(target_os = "macos") {
            assert_eq!(0, getgid("wheel").unwrap());
        } else {
            assert_eq!(0, getgid("root").unwrap());
        }
        assert!(getgid("88888888").is_err());
        assert!(getgid("agroupthatdoesntexist").is_err());
    }

    #[test]
    fn test_uid2usr() {
        assert_eq!("root", uid2usr(0).unwrap());
        assert!(uid2usr(88888888).is_err());
    }

    #[test]
    fn test_gid2grp() {
        if cfg!(target_os = "macos") {
            assert_eq!("wheel", gid2grp(0).unwrap());
        } else {
            assert_eq!("root", gid2grp(0).unwrap());
        }
        assert!(gid2grp(88888888).is_err());
    }
}

#[test]
fn test_invalid_option() {
    new_ucmd()
        .arg("-w").arg("-q").arg("/")
        .fails();
}
