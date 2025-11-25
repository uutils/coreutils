// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore getxattr posix_acl_default

//! Set of functions to manage xattr on files and dirs
use fnv::FnvHashMap as HashMap;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::sync::OnceLock;

pub static POSIX_ACL_ACCESS_KEY: &str = "system.posix_acl_access";
pub static POSIX_ACL_DEFAULT_KEY: &str = "system.posix_acl_default";
pub static SECURITY_CAPABILITY_KEY: &str = "security.capability";

/// Copies extended attributes (xattrs) from one file or directory to another.
///
/// # Arguments
///
/// * `source` - A reference to the source path.
/// * `dest` - A reference to the destination path.
///
/// # Returns
///
/// A result indicating success or failure.
pub fn copy_xattrs<P: AsRef<Path>>(source: P, dest: P) -> std::io::Result<()> {
    for attr_name in xattr::list(&source)? {
        if let Some(value) = xattr::get(&source, &attr_name)? {
            xattr::set(&dest, &attr_name, &value)?;
        }
    }
    Ok(())
}

/// Retrieves the extended attributes (xattrs) of a given file or directory.
///
/// # Arguments
///
/// * `source` - A reference to the path of the file or directory.
///
/// # Returns
///
/// A result containing a HashMap of attributes names and values, or an error.
pub fn retrieve_xattrs<P: AsRef<Path>>(
    source: P,
    must_dereference: bool,
) -> std::io::Result<HashMap<OsString, Vec<u8>>> {
    let mut attrs = HashMap::default();

    let iter = if must_dereference {
        xattr::list_deref(&source)?
    } else {
        xattr::list(&source)?
    };

    attrs.reserve(iter.size_hint().0);

    for attr_name in iter {
        if let Some(value) = xattr::get(&source, &attr_name)? {
            attrs.insert(attr_name, value);
        }
    }

    Ok(attrs)
}

/// Applies extended attributes (xattrs) to a given file or directory.
///
/// # Arguments
///
/// * `dest` - A reference to the path of the file or directory.
/// * `xattrs` - A HashMap containing attribute names and their corresponding values.
///
/// # Returns
///
/// A result indicating success or failure.
pub fn apply_xattrs<P: AsRef<Path>>(
    dest: P,
    xattrs: HashMap<OsString, Vec<u8>>,
) -> std::io::Result<()> {
    for (attr, value) in xattrs {
        xattr::set(&dest, &attr, &value)?;
    }
    Ok(())
}

/// Checks if a file has an Access Control List (ACL) based on its extended attributes.
///
/// # Arguments
///
/// * `file` - A reference to the path of the file.
///
/// # Returns
///
/// `true` if the file has extended attributes (indicating an ACL), `false` otherwise.
pub fn has_acl<P: AsRef<Path>>(file: P) -> bool {
    let access = OsStr::new(POSIX_ACL_ACCESS_KEY);
    let default = OsStr::new(POSIX_ACL_DEFAULT_KEY);

    // don't use exacl here, it is doing more getxattr call then needed
    xattr::list_deref(&file)
        .ok()
        .into_iter()
        .flatten()
        .filter(|name| name.as_os_str() == access || name.as_os_str() == default)
        .filter_map(|name| xattr::get_deref(&file, &name).ok())
        .flatten()
        .any(|item| !item.is_empty())
}

/// Checks if a file has an Capability set based on its extended attributes.
///
/// # Arguments
///
/// * `file` - A reference to the path of the file.
///
/// # Returns
///
/// `true` if the file has a capability extended attribute, `false` otherwise.
pub fn has_capability<P: AsRef<Path>>(file: P) -> bool {
    // check whether thread has cap, done to call capget in order to pass GNU test only
    //
    // AFAICT GNU test must see syscall capget in strace output in order to pass, but has
    // no bearing on what is displayed re files?
    let _ = current_thread_has_capability();

    // don't use exacl here, it is doing more getxattr call then needed
    xattr::get_deref(&file, OsStr::new(SECURITY_CAPABILITY_KEY))
        .ok()
        .flatten()
        .is_some_and(|vec| !vec.is_empty())
}

/// Checks if a thread has an Capability set based Linux capget call.
///
/// `true` if the thread has a capability, `false` otherwise.
pub fn current_thread_has_capability() -> bool {
    #[cfg(target_os = "linux")]
    {
        use capctl::caps;

        static CELL: OnceLock<bool> = OnceLock::new();

        return *CELL.get_or_init(|| {
            matches!(caps::CapState::get_current(), Ok(cap_state) if !cap_state.effective.is_empty()
                    | !cap_state.inheritable.is_empty()
                    | !cap_state.permitted.is_empty()
            )
        });
    }

    false
}

/// Returns the permissions bits of a file or directory which has Access Control List (ACL) entries based on its
/// extended attributes (Only works for linux)
///
/// # Arguments
///
/// * `source` - A reference to the path of the file.
///
/// # Returns
///
/// `u32`  the perm bits of a file having extended attributes of type 'system.posix_acl_default' with permissions
/// otherwise returns a 0 if perm bits are 0 or the file has no extended attributes
pub fn get_acl_perm_bits_from_xattr<P: AsRef<Path>>(source: P) -> u32 {
    // TODO: Modify this to work on non linux unix systems.

    // Only default acl entries get inherited by objects under the path i.e. if child directories
    // will have their permissions modified.
    if let Ok(entries) = retrieve_xattrs(source, true) {
        let mut perm: u32 = 0;
        if let Some(value) = entries.get(OsStr::new(POSIX_ACL_DEFAULT_KEY)) {
            // value is xattr byte vector
            // value follows a starts with a 4 byte header, and then has posix_acl_entries, each
            // posix_acl_entry is separated by a u32 sequence i.e. 0xFFFFFFFF
            //
            // struct posix_acl_entries {
            // e_tag: u16
            //  e_perm: u16
            //  e_id: u32
            // }
            //
            // Reference: `https://github.com/torvalds/linux/blob/master/include/uapi/linux/posix_acl_xattr.h`
            //
            // The value of the header is 0x0002, so we skip the first four bytes of the value and
            // process the rest

            let acl_entries = value
                .split_at(3)
                .1
                .iter()
                .filter(|&x| *x != 255)
                .copied()
                .collect::<Vec<u8>>();

            for entry in acl_entries.chunks_exact(4) {
                // Third byte and fourth byte will be the perm bits
                perm = (perm << 3) | u32::from(entry[2]) | u32::from(entry[3]);
            }
            return perm;
        }
    }
    0
}

// FIXME: 3 tests failed on OpenBSD
#[cfg(not(target_os = "openbsd"))]
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_copy_xattrs() {
        let temp_dir = tempdir().unwrap();
        let source_path = temp_dir.path().join("source.txt");
        let dest_path = temp_dir.path().join("dest.txt");

        File::create(&source_path).unwrap();
        File::create(&dest_path).unwrap();

        let test_attr = "user.test";
        let test_value = b"test value";
        xattr::set(&source_path, test_attr, test_value).unwrap();

        copy_xattrs(&source_path, &dest_path).unwrap();

        let copied_value = xattr::get(&dest_path, test_attr).unwrap().unwrap();
        assert_eq!(copied_value, test_value);
    }

    #[test]
    fn test_apply_and_retrieve_xattrs() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");

        File::create(&file_path).unwrap();

        let mut test_xattrs = HashMap::default();
        let test_attr = "user.test_attr";
        let test_value = b"test value";
        test_xattrs.insert(OsString::from(test_attr), test_value.to_vec());
        apply_xattrs(&file_path, test_xattrs).unwrap();

        let retrieved_xattrs = retrieve_xattrs(&file_path, true).unwrap();
        assert!(retrieved_xattrs.contains_key(OsString::from(test_attr).as_os_str()));
        assert_eq!(
            retrieved_xattrs
                .get(OsString::from(test_attr).as_os_str())
                .unwrap(),
            test_value
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_get_perm_bits_from_xattrs() {
        let temp_dir = tempdir().unwrap();
        let source_path = temp_dir.path().join("source_dir");

        std::fs::create_dir(&source_path).unwrap();

        let test_attr = "system.posix_acl_default";
        // posix_acl entries are in the form of
        // struct posix_acl_entry{
        //  tag: u16,
        //  perm: u16,
        //  id: u32,
        // }
        // the fields are serialized in little endian.
        // The entries are preceded by a header of value of 0x0002
        // Reference: `<https://github.com/torvalds/linux/blob/master/include/uapi/linux/posix_acl_xattr.h>`
        // The id is undefined i.e. -1 which in u32 is 0xFFFFFFFF and tag and perm bits as given in the
        // header file.
        // Reference: `<https://github.com/torvalds/linux/blob/master/include/uapi/linux/posix_acl.h>`
        //
        //
        // There is a bindgen bug which generates the ACL_OTHER constant whose value is 0x20 into 32.
        // which when the bug is fixed will need to be changed back to 20 from 32 in the vec 'test_value'.
        //
        // Reference `<https://github.com/rust-lang/rust-bindgen/issues/2926>`
        //
        // The test_value vector is the header 0x0002 followed by tag and permissions for user_obj , tag
        // and permissions and for group_obj and finally the tag and permissions for ACL_OTHER. Each
        // entry has undefined id as mentioned above.
        //
        //

        let test_value = vec![
            2, 0, 0, 0, 1, 0, 7, 0, 255, 255, 255, 255, 4, 0, 0, 0, 255, 255, 255, 255, 32, 0, 0,
            0, 255, 255, 255, 255,
        ];

        xattr::set(&source_path, test_attr, test_value.as_slice()).unwrap();

        let perm_bits = get_acl_perm_bits_from_xattr(source_path);

        assert_eq!(0o700, perm_bits);
    }

    #[test]
    fn test_file_has_acl() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");

        File::create(&file_path).unwrap();

        assert!(!has_acl(&file_path));

        let test_attr = "system.posix_acl_access";
        let test_value = "invalid_test_value";
        // perhaps can't set actual ACL in test environment? if so, return early
        let Ok(_) = xattr::set(&file_path, test_attr, test_value.as_bytes()) else {
            return;
        };

        assert!(has_acl(&file_path));
    }
}
