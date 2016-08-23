use common::util::*;

extern crate uu_chown;
pub use self::uu_chown::*;


#[cfg(test)]
mod test_passgrp {
    use super::uu_chown::entries::{usr2uid,grp2gid,uid2usr,gid2grp};

    #[test]
    fn test_usr2uid() {
        assert_eq!(0, usr2uid("root").unwrap());
        assert!(usr2uid("88888888").is_err());
        assert!(usr2uid("auserthatdoesntexist").is_err());
    }

    #[test]
    fn test_grp2gid() {
        if cfg!(target_os = "macos") {
            assert_eq!(0, grp2gid("wheel").unwrap());
        } else {
            assert_eq!(0, grp2gid("root").unwrap());
        }
        assert!(grp2gid("88888888").is_err());
        assert!(grp2gid("agroupthatdoesntexist").is_err());
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
    new_ucmd!()
        .arg("-w").arg("-q").arg("/")
        .fails();
}
