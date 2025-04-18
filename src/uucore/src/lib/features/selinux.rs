// This file is part of the uutils uucore package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::path::Path;

use selinux::SecurityContext;

#[derive(Debug)]
pub enum Error {
    SELinuxNotEnabled,
    FileOpenFailure,
    ContextRetrievalFailure,
    ContextConversionFailure,
}

impl From<Error> for i32 {
    fn from(error: Error) -> i32 {
        match error {
            Error::SELinuxNotEnabled => 1,
            Error::FileOpenFailure => 2,
            Error::ContextRetrievalFailure => 3,
            Error::ContextConversionFailure => 4,
        }
    }
}

/// Checks if SELinux is enabled on the system.
///
/// This function verifies whether the kernel has SELinux support enabled.
pub fn is_selinux_enabled() -> bool {
    selinux::kernel_support() != selinux::KernelSupport::Unsupported
}

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
    if !is_selinux_enabled() {
        return Err("SELinux is not enabled on this system".to_string());
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

/// Gets the SELinux security context for the given filesystem path.
///
/// Retrieves the security context of the specified filesystem path if SELinux is enabled
/// on the system.
///
/// # Arguments
///
/// * `path` - Filesystem path for which to retrieve the SELinux context.
///
/// # Returns
///
/// * `Ok(String)` - The SELinux context string if successfully retrieved. Returns an empty
///   string if no context was found.
/// * `Err(Error)` - An error variant indicating the type of failure:
///   - `Error::SELinuxNotEnabled` - SELinux is not enabled on the system.
///   - `Error::FileOpenFailure` - Failed to open the specified file.
///   - `Error::ContextRetrievalFailure` - Failed to retrieve the security context.
///   - `Error::ContextConversionFailure` - Failed to convert the security context to a string.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use uucore::selinux::{get_selinux_security_context, Error};
///
/// // Get the SELinux context for a file
/// match get_selinux_security_context(Path::new("/path/to/file")) {
///     Ok(context) => {
///         if context.is_empty() {
///             println!("No SELinux context found for the file");
///         } else {
///             println!("SELinux context: {}", context);
///         }
///     },
///     Err(Error::SELinuxNotEnabled) => println!("SELinux is not enabled on this system"),
///     Err(Error::FileOpenFailure) => println!("Failed to open the file"),
///     Err(Error::ContextRetrievalFailure) => println!("Failed to retrieve the security context"),
///     Err(Error::ContextConversionFailure) => println!("Failed to convert the security context to a string"),
/// }
/// ```
pub fn get_selinux_security_context(path: &Path) -> Result<String, Error> {
    if !is_selinux_enabled() {
        return Err(Error::SELinuxNotEnabled);
    }

    let f = std::fs::File::open(path).map_err(|_| Error::FileOpenFailure)?;

    // Get the security context of the file
    let context = match SecurityContext::of_file(&f, false) {
        Ok(Some(ctx)) => ctx,
        Ok(None) => return Ok(String::new()), // No context found, return empty string
        Err(_) => return Err(Error::ContextRetrievalFailure),
    };

    let context_c_string = context
        .to_c_string()
        .map_err(|_| Error::ContextConversionFailure)?;

    if let Some(c_str) = context_c_string {
        // Convert the C string to a Rust String
        Ok(c_str.to_string_lossy().to_string())
    } else {
        Ok(String::new())
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

    #[test]
    fn test_is_selinux_enabled_runtime_behavior() {
        let result = is_selinux_enabled();

        match selinux::kernel_support() {
            selinux::KernelSupport::Unsupported => {
                assert!(!result, "Expected false when SELinux is not supported");
            }
            _ => {
                assert!(result, "Expected true when SELinux is supported");
            }
        }
    }

    #[test]
    fn test_get_selinux_security_context() {
        let tmpfile = NamedTempFile::new().expect("Failed to create tempfile");
        let path = tmpfile.path();

        std::fs::write(path, b"test content").expect("Failed to write to tempfile");

        let result = get_selinux_security_context(path);

        if result.is_ok() {
            println!("Retrieved SELinux context: {}", result.unwrap());
        } else {
            let err = result.unwrap_err();

            // Valid error types
            match err {
                Error::SELinuxNotEnabled => assert!(true, "SELinux not supported"),
                Error::ContextRetrievalFailure => assert!(true, "Context retrieval failure"),
                Error::ContextConversionFailure => assert!(true, "Context conversion failure"),
                Error::FileOpenFailure => {
                    panic!("File open failure occurred despite file being created")
                }
            }
        }
    }

    #[test]
    fn test_get_selinux_context_nonexistent_file() {
        let path = Path::new("/nonexistent/file/that/does/not/exist");

        let result = get_selinux_security_context(path);

        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), Error::FileOpenFailure),
            "Expected file open error for nonexistent file"
        );
    }
}
