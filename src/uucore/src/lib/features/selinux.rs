// This file is part of the uutils uucore package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::path::Path;

use selinux::SecurityContext;

/// Sets the SELinux security context for the given filesystem path.
///
/// If a specific context is provided, it attempts to set this context explicitly.
/// Otherwise, it applies the default SELinux context for the provided path.
///
/// # Arguments
///
/// * `path` - Filesystem path on which to set the SELinux context.
/// * `context` - Optional SELinux context string to explicitly set.
///
/// # Errors
///
/// Returns an error if:
/// - SELinux is not enabled on the system.
/// - The provided context is invalid or cannot be applied.
/// - The default SELinux context cannot be set.
///
/// # Examples
///
/// Setting default context:
/// ```
/// use std::path::Path;
/// use uucore::selinux::set_selinux_security_context;
///
/// // Set the default SELinux context for a file
/// let result = set_selinux_security_context(Path::new("/path/to/file"), None);
/// if let Err(err) = result {
///     eprintln!("Failed to set default context: {}", err);
/// }
/// ```
///
/// Setting specific context:
/// ```
/// use std::path::Path;
/// use uucore::selinux::set_selinux_security_context;
///
/// // Set a specific SELinux context for a file
/// let context = String::from("unconfined_u:object_r:user_home_t:s0");
/// let result = set_selinux_security_context(Path::new("/path/to/file"), Some(&context));
/// if let Err(err) = result {
///     eprintln!("Failed to set context: {}", err);
/// }
/// ```
pub fn set_selinux_security_context(path: &Path, context: Option<&String>) -> Result<(), String> {
    // Check if SELinux is enabled on the system
    if selinux::kernel_support() == selinux::KernelSupport::Unsupported {
        return Err("SELinux is not enabled on this system".into());
    }

    if let Some(ctx_str) = context {
        // Create a CString from the provided context string
        let c_context = std::ffi::CString::new(ctx_str.as_str())
            .map_err(|_| "Invalid context string (contains null bytes)".to_string())?;

        // Convert the CString into an SELinux security context
        let security_context = selinux::OpaqueSecurityContext::from_c_str(&c_context)
            .map_err(|e| format!("Failed to create security context: {}", e))?;

        // Set the provided security context on the specified path
        SecurityContext::from_c_str(
            &security_context.to_c_string().map_err(|e| e.to_string())?,
            false,
        )
        .set_for_path(path, false, false)
        .map_err(|e| format!("Failed to set context: {}", e))
    } else {
        // If no context provided, set the default SELinux context for the path
        SecurityContext::set_default_for_path(path)
            .map_err(|e| format!("Failed to set default context: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_selinux_context_setting() {
        let tmpfile = NamedTempFile::new().expect("Failed to create tempfile");
        let path = tmpfile.path();

        let result = set_selinux_security_context(path, None);

        if result.is_ok() {
            // SELinux enabled and successfully set default context
            assert!(true, "Successfully set SELinux context");
        } else {
            let err = result.unwrap_err();
            let valid_errors = [
                "SELinux is not enabled on this system",
                &format!(
                    "Failed to set default context: selinux_lsetfilecon_default() failed on path '{}'",
                    path.display()
                ),
            ];

            assert!(
                valid_errors.contains(&err.as_str()),
                "Unexpected error message: {}",
                err
            );
        }
    }

    #[test]
    fn test_invalid_context_string_error() {
        let tmpfile = NamedTempFile::new().expect("Failed to create tempfile");
        let path = tmpfile.path();

        // Pass a context string containing a null byte to trigger CString::new error
        let invalid_context = String::from("invalid\0context");
        let result = set_selinux_security_context(path, Some(&invalid_context));

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Invalid context string (contains null bytes)"
        );
    }
}
