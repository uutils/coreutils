// This file is part of the uutils uucore package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::error::Error;
use std::path::Path;

use selinux::SecurityContext;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SeLinuxError {
    #[error("SELinux is not enabled on this system")]
    SELinuxNotEnabled,

    #[error("failed to open the file: {0}")]
    FileOpenFailure(String),

    #[error("failed to retrieve the security context: {0}")]
    ContextRetrievalFailure(String),

    #[error("failed to set default file creation context to '{0}': {1}")]
    ContextSetFailure(String, String),

    #[error("failed to set default file creation context to '{0}': {1}")]
    ContextConversionFailure(String, String),
}

impl From<SeLinuxError> for i32 {
    fn from(error: SeLinuxError) -> i32 {
        match error {
            SeLinuxError::SELinuxNotEnabled => 1,
            SeLinuxError::FileOpenFailure(_) => 2,
            SeLinuxError::ContextRetrievalFailure(_) => 3,
            SeLinuxError::ContextSetFailure(_, _) => 4,
            SeLinuxError::ContextConversionFailure(_, _) => 5,
        }
    }
}

/// Checks if SELinux is enabled on the system.
///
/// This function verifies whether the kernel has SELinux support enabled.
pub fn is_selinux_enabled() -> bool {
    selinux::kernel_support() != selinux::KernelSupport::Unsupported
}

/// Returns a string describing the error and its causes.
fn selinux_error_description(mut error: &dyn Error) -> String {
    let mut description = String::new();
    while let Some(source) = error.source() {
        let error_text = source.to_string();
        // Check if this is an OS error and trim it
        if let Some(idx) = error_text.find(" (os error ") {
            description.push_str(&error_text[..idx]);
        } else {
            description.push_str(&error_text);
        }
        error = source;
    }
    description
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
pub fn set_selinux_security_context(
    path: &Path,
    context: Option<&String>,
) -> Result<(), SeLinuxError> {
    if !is_selinux_enabled() {
        return Err(SeLinuxError::SELinuxNotEnabled);
    }

    if let Some(ctx_str) = context {
        // Create a CString from the provided context string
        let c_context = std::ffi::CString::new(ctx_str.as_str()).map_err(|e| {
            SeLinuxError::ContextConversionFailure(
                ctx_str.to_string(),
                selinux_error_description(&e),
            )
        })?;

        // Convert the CString into an SELinux security context
        let security_context =
            selinux::OpaqueSecurityContext::from_c_str(&c_context).map_err(|e| {
                SeLinuxError::ContextConversionFailure(
                    ctx_str.to_string(),
                    selinux_error_description(&e),
                )
            })?;

        // Set the provided security context on the specified path
        SecurityContext::from_c_str(
            &security_context.to_c_string().map_err(|e| {
                SeLinuxError::ContextConversionFailure(
                    ctx_str.to_string(),
                    selinux_error_description(&e),
                )
            })?,
            false,
        )
        .set_for_path(path, false, false)
        .map_err(|e| {
            SeLinuxError::ContextSetFailure(ctx_str.to_string(), selinux_error_description(&e))
        })
    } else {
        // If no context provided, set the default SELinux context for the path
        SecurityContext::set_default_for_path(path).map_err(|e| {
            SeLinuxError::ContextSetFailure(String::new(), selinux_error_description(&e))
        })
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
/// * `Err(SeLinuxError)` - An error variant indicating the type of failure:
///   - `SeLinuxError::SELinuxNotEnabled` - SELinux is not enabled on the system.
///   - `SeLinuxError::FileOpenFailure` - Failed to open the specified file.
///   - `SeLinuxError::ContextRetrievalFailure` - Failed to retrieve the security context.
///   - `SeLinuxError::ContextConversionFailure` - Failed to convert the security context to a string.
///   - `SeLinuxError::ContextSetFailure` - Failed to set the security context.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use uucore::selinux::{get_selinux_security_context, SeLinuxError};
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
///     Err(SeLinuxError::SELinuxNotEnabled) => println!("SELinux is not enabled on this system"),
///     Err(SeLinuxError::FileOpenFailure(e)) => println!("Failed to open the file: {}", e),
///     Err(SeLinuxError::ContextRetrievalFailure(e)) => println!("Failed to retrieve the security context: {}", e),
///     Err(SeLinuxError::ContextConversionFailure(ctx, e)) => println!("Failed to convert context '{}': {}", ctx, e),
///     Err(SeLinuxError::ContextSetFailure(ctx, e)) => println!("Failed to set context '{}': {}", ctx, e),
/// }
/// ```
pub fn get_selinux_security_context(path: &Path) -> Result<String, SeLinuxError> {
    if !is_selinux_enabled() {
        return Err(SeLinuxError::SELinuxNotEnabled);
    }

    let f = std::fs::File::open(path)
        .map_err(|e| SeLinuxError::FileOpenFailure(selinux_error_description(&e)))?;

    // Get the security context of the file
    let context = match SecurityContext::of_file(&f, false) {
        Ok(Some(ctx)) => ctx,
        Ok(None) => return Ok(String::new()), // No context found, return empty string
        Err(e) => {
            return Err(SeLinuxError::ContextRetrievalFailure(
                selinux_error_description(&e),
            ));
        }
    };

    let context_c_string = context.to_c_string().map_err(|e| {
        SeLinuxError::ContextConversionFailure(String::new(), selinux_error_description(&e))
    })?;

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

        if !is_selinux_enabled() {
            let result = set_selinux_security_context(path, None);
            assert!(result.is_err(), "Expected error when SELinux is disabled");
            match result.unwrap_err() {
                SeLinuxError::SELinuxNotEnabled => {
                    // This is the expected error when SELinux is not enabled
                }
                err => panic!("Expected SELinuxNotEnabled error but got: {}", err),
            }
            return;
        }

        let default_result = set_selinux_security_context(path, None);
        assert!(
            default_result.is_ok(),
            "Failed to set default context: {:?}",
            default_result.err()
        );

        let context = get_selinux_security_context(path).expect("Failed to get context");
        assert!(
            !context.is_empty(),
            "Expected non-empty context after setting default context"
        );

        let test_context = String::from("system_u:object_r:tmp_t:s0");
        let explicit_result = set_selinux_security_context(path, Some(&test_context));

        if explicit_result.is_ok() {
            let new_context = get_selinux_security_context(path)
                .expect("Failed to get context after setting explicit context");

            assert!(
                new_context.contains("tmp_t"),
                "Expected context to contain 'tmp_t', but got: {}",
                new_context
            );
        } else {
            println!(
                "Note: Could not set explicit context {:?}",
                explicit_result.err()
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
        if let Err(err) = result {
            match err {
                SeLinuxError::ContextConversionFailure(ctx, msg) => {
                    assert_eq!(ctx, "invalid\0context");
                    assert!(
                        msg.contains("nul byte"),
                        "Error message should mention nul byte"
                    );
                }
                _ => panic!("Expected ContextConversionFailure error but got: {}", err),
            }
        }
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
            let context = result.unwrap();
            println!("Retrieved SELinux context: {}", context);

            assert!(
                is_selinux_enabled(),
                "Got a successful context result but SELinux is not enabled"
            );

            if !context.is_empty() {
                assert!(
                    context.contains(':'),
                    "SELinux context '{}' doesn't match expected format",
                    context
                );
            }
        } else {
            let err = result.unwrap_err();

            match err {
                SeLinuxError::SELinuxNotEnabled => {
                    assert!(
                        !is_selinux_enabled(),
                        "Got SELinuxNotEnabled error, but is_selinux_enabled() returned true"
                    );
                }
                SeLinuxError::ContextRetrievalFailure(e) => {
                    assert!(
                        is_selinux_enabled(),
                        "Got ContextRetrievalFailure when SELinux is not enabled"
                    );
                    assert!(!e.is_empty(), "Error message should not be empty");
                    println!("Context retrieval failure: {}", e);
                }
                SeLinuxError::ContextConversionFailure(ctx, e) => {
                    assert!(
                        is_selinux_enabled(),
                        "Got ContextConversionFailure when SELinux is not enabled"
                    );
                    assert!(!e.is_empty(), "Error message should not be empty");
                    println!("Context conversion failure for '{}': {}", ctx, e);
                }
                SeLinuxError::ContextSetFailure(ctx, e) => {
                    assert!(!e.is_empty(), "Error message should not be empty");
                    println!("Context conversion failure for '{}': {}", ctx, e);
                }
                SeLinuxError::FileOpenFailure(e) => {
                    assert!(
                        Path::new(path).exists(),
                        "File open failure occurred despite file being created: {}",
                        e
                    );
                }
            }
        }
    }

    #[test]
    fn test_get_selinux_context_nonexistent_file() {
        let path = Path::new("/nonexistent/file/that/does/not/exist");

        let result = get_selinux_security_context(path);

        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                SeLinuxError::FileOpenFailure(e) => {
                    assert!(
                        e.contains("No such file"),
                        "Error should mention file not found"
                    );
                }
                _ => panic!("Expected FileOpenFailure error but got: {}", err),
            }
        }
    }
}
