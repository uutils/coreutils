// This file is part of the uutils uucore package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Set of functions to manage SELinux security contexts

use std::error::Error;
use std::path::Path;

use crate::translate;
use selinux::SecurityContext;
use thiserror::Error;

use crate::error::UError;

#[derive(Debug, Error)]
pub enum SeLinuxError {
    #[error("{}", translate!("selinux-error-not-enabled"))]
    SELinuxNotEnabled,

    #[error("{}", translate!("selinux-error-file-open-failure", "error" => .0.clone()))]
    FileOpenFailure(String),

    #[error("{}", translate!("selinux-error-context-retrieval-failure", "error" => .0.clone()))]
    ContextRetrievalFailure(String),

    #[error("{}", translate!("selinux-error-context-set-failure", "context" => .0.clone(), "error" => .1.clone()))]
    ContextSetFailure(String, String),

    #[error("{}", translate!("selinux-error-context-conversion-failure", "context" => .0.clone(), "error" => .1.clone()))]
    ContextConversionFailure(String, String),
}

impl UError for SeLinuxError {
    fn code(&self) -> i32 {
        match self {
            Self::SELinuxNotEnabled => 1,
            Self::FileOpenFailure(_) => 2,
            Self::ContextRetrievalFailure(_) => 3,
            Self::ContextSetFailure(_, _) => 4,
            Self::ContextConversionFailure(_, _) => 5,
        }
    }
}

impl From<SeLinuxError> for i32 {
    fn from(error: SeLinuxError) -> Self {
        error.code()
    }
}

/// Checks if SELinux is enabled on the system.
///
/// This function verifies whether the kernel has SELinux support enabled.
/// Note: libselinux internally caches this value, so no additional caching is needed.
pub fn is_selinux_enabled() -> bool {
    selinux::kernel_support() != selinux::KernelSupport::Unsupported
}

/// Returns a string describing the error and its causes.
pub fn selinux_error_description(mut error: &dyn Error) -> String {
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
///     eprintln!("Failed to set default context: {err}");
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
///     eprintln!("Failed to set context: {err}");
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
                ctx_str.to_owned(),
                selinux_error_description(&e),
            )
        })?;

        // Convert the CString into an SELinux security context
        let security_context =
            selinux::OpaqueSecurityContext::from_c_str(&c_context).map_err(|e| {
                SeLinuxError::ContextConversionFailure(
                    ctx_str.to_owned(),
                    selinux_error_description(&e),
                )
            })?;

        // Set the provided security context on the specified path
        SecurityContext::from_c_str(
            &security_context.to_c_string().map_err(|e| {
                SeLinuxError::ContextConversionFailure(
                    ctx_str.to_owned(),
                    selinux_error_description(&e),
                )
            })?,
            false,
        )
        .set_for_path(path, false, false)
        .map_err(|e| {
            SeLinuxError::ContextSetFailure(ctx_str.to_owned(), selinux_error_description(&e))
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
/// match get_selinux_security_context(Path::new("/path/to/file"), false) {
///     Ok(context) => {
///         if context.is_empty() {
///             println!("No SELinux context found for the file");
///         } else {
///             println!("SELinux context: {context}");
///         }
///     },
///     Err(SeLinuxError::SELinuxNotEnabled) => println!("SELinux is not enabled on this system"),
///     Err(SeLinuxError::FileOpenFailure(e)) => println!("Failed to open the file: {e}"),
///     Err(SeLinuxError::ContextRetrievalFailure(e)) => println!("Failed to retrieve the security context: {e}"),
///     Err(SeLinuxError::ContextConversionFailure(ctx, e)) => println!("Failed to convert context '{ctx}': {e}"),
///     Err(SeLinuxError::ContextSetFailure(ctx, e)) => println!("Failed to set context '{ctx}': {e}"),
/// }
/// ```
pub fn get_selinux_security_context(
    path: &Path,
    follow_symbolic_links: bool,
) -> Result<String, SeLinuxError> {
    if !is_selinux_enabled() {
        return Err(SeLinuxError::SELinuxNotEnabled);
    }

    // Get the security context of the file
    let context = match SecurityContext::of_path(path, follow_symbolic_links, false) {
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

/// Compares SELinux security contexts of two filesystem paths.
///
/// This function retrieves and compares the SELinux security contexts of two paths.
/// If the contexts differ or an error occurs during retrieval, it returns true.
///
/// # Arguments
///
/// * `from_path` - Source filesystem path.
/// * `to_path` - Destination filesystem path.
///
/// # Returns
///
/// * `true` - If contexts differ, cannot be retrieved, or if SELinux is not enabled.
/// * `false` - If contexts are the same.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use uucore::selinux::contexts_differ;
///
/// // Check if contexts differ between two files
/// let differ = contexts_differ(Path::new("/path/to/source"), Path::new("/path/to/destination"));
/// if differ {
///     println!("Files have different SELinux contexts");
/// } else {
///     println!("Files have the same SELinux context");
/// }
/// ```
pub fn contexts_differ(from_path: &Path, to_path: &Path) -> bool {
    if !is_selinux_enabled() {
        return true;
    }

    // Check if SELinux contexts differ
    match (
        selinux::SecurityContext::of_path(from_path, false, false),
        selinux::SecurityContext::of_path(to_path, false, false),
    ) {
        (Ok(Some(from_ctx)), Ok(Some(to_ctx))) => {
            // Convert contexts to CString and compare
            match (from_ctx.to_c_string(), to_ctx.to_c_string()) {
                (Ok(Some(from_c_str)), Ok(Some(to_c_str))) => {
                    from_c_str.to_string_lossy() != to_c_str.to_string_lossy()
                }
                // If contexts couldn't be converted to CString or were None, consider them different
                _ => true,
            }
        }
        // If either context is None or an error occurred, assume contexts differ
        _ => true,
    }
}

/// Preserves the SELinux security context from one filesystem path to another.
///
/// This function copies the security context from the source path to the destination path.
/// If SELinux is not enabled, or if the source has no context, the function returns success
/// without making any changes.
///
/// # Arguments
///
/// * `from_path` - Source filesystem path from which to copy the SELinux context.
/// * `to_path` - Destination filesystem path to which the context should be applied.
///
/// # Returns
///
/// * `Ok(())` - If the context was successfully preserved or if SELinux is not enabled.
/// * `Err(SeLinuxError)` - If an error occurred during context retrieval or application.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use uucore::selinux::preserve_security_context;
///
/// // Preserve the SELinux context from source to destination
/// match preserve_security_context(Path::new("/path/to/source"), Path::new("/path/to/destination")) {
///     Ok(_) => println!("Context preserved successfully (or SELinux is not enabled)"),
///     Err(err) => eprintln!("Failed to preserve context: {err}"),
/// }
/// ```
pub fn preserve_security_context(from_path: &Path, to_path: &Path) -> Result<(), SeLinuxError> {
    // If SELinux is not enabled, return success without doing anything
    if !is_selinux_enabled() {
        return Err(SeLinuxError::SELinuxNotEnabled);
    }

    // Get context from the source path
    let context = get_selinux_security_context(from_path, false)?;

    // If no context was found, just return success (nothing to preserve)
    if context.is_empty() {
        return Ok(());
    }

    // Apply the context to the destination path
    set_selinux_security_context(to_path, Some(&context))
}

/// Gets the SELinux security context for a file using getfattr.
///
/// This function is primarily used for testing purposes to verify that SELinux
/// contexts have been properly set on files. It uses the `getfattr` command
/// to retrieve the security.selinux extended attribute.
///
/// # Arguments
///
/// * `f` - The file path as a string.
///
/// # Returns
///
/// Returns the SELinux context string extracted from the getfattr output.
/// If the context cannot be retrieved, the function will panic.
///
/// # Panics
///
/// This function will panic if:
/// - The `getfattr` command fails to execute
/// - The `getfattr` command returns a non-zero exit status
///
/// # Examples
///
/// ```no_run
/// use uucore::selinux::get_getfattr_output;
///
/// let context = get_getfattr_output("/path/to/file");
/// println!("SELinux context: {}", context);
/// ```
pub fn get_getfattr_output(f: &str) -> String {
    use std::process::Command;

    let getfattr_output = Command::new("getfattr")
        .arg(f)
        .arg("-n")
        .arg("security.selinux")
        .output()
        .expect("Failed to run `getfattr` on the destination file");
    println!("{getfattr_output:?}");
    assert!(
        getfattr_output.status.success(),
        "getfattr did not run successfully: {}",
        String::from_utf8_lossy(&getfattr_output.stderr)
    );

    String::from_utf8_lossy(&getfattr_output.stdout)
        .split('"')
        .nth(1)
        .unwrap_or("")
        .to_string()
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
                err => panic!("Expected SELinuxNotEnabled error but got: {err}"),
            }
            return;
        }

        let default_result = set_selinux_security_context(path, None);
        assert!(
            default_result.is_ok(),
            "Failed to set default context: {:?}",
            default_result.err()
        );

        let context = get_selinux_security_context(path, false).expect("Failed to get context");
        assert!(
            !context.is_empty(),
            "Expected non-empty context after setting default context"
        );

        let test_context = String::from("system_u:object_r:tmp_t:s0");
        let explicit_result = set_selinux_security_context(path, Some(&test_context));

        if explicit_result.is_ok() {
            let new_context = get_selinux_security_context(path, false)
                .expect("Failed to get context after setting explicit context");

            assert!(
                new_context.contains("tmp_t"),
                "Expected context to contain 'tmp_t', but got: {new_context}"
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
        if !is_selinux_enabled() {
            println!("test skipped: Kernel has no support for SElinux context");
            return;
        }
        // Pass a context string containing a null byte to trigger CString::new error
        let invalid_context = String::from("invalid\0context");
        let result = set_selinux_security_context(path, Some(&invalid_context));

        assert!(result.is_err());
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
        if !is_selinux_enabled() {
            println!("test skipped: Kernel has no support for SElinux context");
            return;
        }
        std::fs::write(path, b"test content").expect("Failed to write to tempfile");

        let result = get_selinux_security_context(path, false);

        match result {
            Ok(context) => {
                println!("Retrieved SELinux context: {context}");

                assert!(
                    is_selinux_enabled(),
                    "Got a successful context result but SELinux is not enabled"
                );

                if !context.is_empty() {
                    assert!(
                        context.contains(':'),
                        "SELinux context '{context}' doesn't match expected format"
                    );
                }
            }
            Err(SeLinuxError::SELinuxNotEnabled) => {
                assert!(
                    !is_selinux_enabled(),
                    "Got SELinuxNotEnabled error, but is_selinux_enabled() returned true"
                );
            }
            Err(SeLinuxError::ContextRetrievalFailure(e)) => {
                assert!(
                    is_selinux_enabled(),
                    "Got ContextRetrievalFailure when SELinux is not enabled"
                );
                assert!(!e.is_empty(), "Error message should not be empty");
                println!("Context retrieval failure: {e}");
            }
            Err(SeLinuxError::ContextConversionFailure(ctx, e)) => {
                assert!(
                    is_selinux_enabled(),
                    "Got ContextConversionFailure when SELinux is not enabled"
                );
                assert!(!e.is_empty(), "Error message should not be empty");
                println!("Context conversion failure for '{ctx}': {e}");
            }
            Err(SeLinuxError::ContextSetFailure(ctx, e)) => {
                assert!(!e.is_empty(), "Error message should not be empty");
                println!("Context conversion failure for '{ctx}': {e}");
            }
            Err(SeLinuxError::FileOpenFailure(e)) => {
                assert!(
                    Path::new(path).exists(),
                    "File open failure occurred despite file being created: {e}"
                );
            }
        }
    }

    #[test]
    fn test_get_selinux_context_nonexistent_file() {
        let path = Path::new("/nonexistent/file/that/does/not/exist");
        if !is_selinux_enabled() {
            println!("test skipped: Kernel has no support for SElinux context");
            return;
        }
        let result = get_selinux_security_context(path, false);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_selinux_context_symlink() {
        use std::os::unix::fs::symlink;
        use tempfile::tempdir;

        if !is_selinux_enabled() {
            println!("test skipped: Kernel has no support for SElinux context");
            return;
        }

        let tmp_dir = tempdir().expect("Failed to create temporary directory");
        let dir_path = tmp_dir.path();

        // Create a normal file
        let file_path = dir_path.join("file");
        std::fs::File::create(&file_path).expect("Failed to create file");

        // Create a symlink to the file
        let symlink_path = dir_path.join("symlink");
        symlink(&file_path, &symlink_path).expect("Failed to create symlink");

        // Set a different context for the file (but not the symlink)
        let file_context = String::from("system_u:object_r:user_tmp_t:s0");
        set_selinux_security_context(&file_path, Some(&file_context))
            .expect("Failed to set security context.");

        // Context must be different if we don't follow the link
        let file_context = get_selinux_security_context(&file_path, false)
            .expect("Failed to get security context.");
        let symlink_context = get_selinux_security_context(&symlink_path, false)
            .expect("Failed to get security context.");
        assert_ne!(file_context, symlink_context);

        // Context must be the same if we follow the link
        let symlink_follow_context = get_selinux_security_context(&symlink_path, true)
            .expect("Failed to get security context.");
        assert_eq!(file_context, symlink_follow_context);
    }

    #[test]
    fn test_get_selinux_context_fifo() {
        use tempfile::tempdir;

        if !is_selinux_enabled() {
            println!("test skipped: Kernel has no support for SElinux context");
            return;
        }

        let tmp_dir = tempdir().expect("Failed to create temporary directory");
        let dir_path = tmp_dir.path();

        // Create a FIFO (pipe)
        let fifo_path = dir_path.join("my_fifo");
        crate::fs::make_fifo(&fifo_path).expect("Failed to create FIFO");

        // Just getting a context is good enough
        get_selinux_security_context(&fifo_path, false).expect("Cannot get fifo context");
    }

    #[test]
    fn test_contexts_differ() {
        let file1 = NamedTempFile::new().expect("Failed to create first tempfile");
        let file2 = NamedTempFile::new().expect("Failed to create second tempfile");
        let path1 = file1.path();
        let path2 = file2.path();

        std::fs::write(path1, b"content for file 1").expect("Failed to write to first tempfile");
        std::fs::write(path2, b"content for file 2").expect("Failed to write to second tempfile");

        if !is_selinux_enabled() {
            assert!(
                contexts_differ(path1, path2),
                "contexts_differ should return true when SELinux is not enabled"
            );
            return;
        }

        let test_context = String::from("system_u:object_r:tmp_t:s0");
        let result1 = set_selinux_security_context(path1, Some(&test_context));
        let result2 = set_selinux_security_context(path2, Some(&test_context));

        if result1.is_ok() && result2.is_ok() {
            assert!(
                !contexts_differ(path1, path2),
                "Contexts should not differ when the same context is set on both files"
            );

            let different_context = String::from("system_u:object_r:user_tmp_t:s0");
            if set_selinux_security_context(path2, Some(&different_context)).is_ok() {
                assert!(
                    contexts_differ(path1, path2),
                    "Contexts should differ when different contexts are set"
                );
            }
        } else {
            println!(
                "Note: Couldn't set SELinux contexts to test differences. This is expected if the test doesn't have sufficient permissions."
            );
            assert!(
                contexts_differ(path1, path2),
                "Contexts should differ when different contexts are set"
            );
        }

        let nonexistent_path = Path::new("/nonexistent/file/path");
        assert!(
            contexts_differ(path1, nonexistent_path),
            "contexts_differ should return true when one path doesn't exist"
        );
    }

    #[test]
    fn test_preserve_security_context() {
        let source_file = NamedTempFile::new().expect("Failed to create source tempfile");
        let dest_file = NamedTempFile::new().expect("Failed to create destination tempfile");
        let source_path = source_file.path();
        let dest_path = dest_file.path();

        std::fs::write(source_path, b"source content").expect("Failed to write to source tempfile");
        std::fs::write(dest_path, b"destination content")
            .expect("Failed to write to destination tempfile");

        if !is_selinux_enabled() {
            let result = preserve_security_context(source_path, dest_path);
            assert!(
                result.is_err(),
                "preserve_security_context should fail when SELinux is not enabled"
            );
            return;
        }

        let source_context = String::from("system_u:object_r:tmp_t:s0");
        let result = set_selinux_security_context(source_path, Some(&source_context));

        if result.is_ok() {
            let preserve_result = preserve_security_context(source_path, dest_path);
            assert!(
                preserve_result.is_ok(),
                "Failed to preserve context: {:?}",
                preserve_result.err()
            );

            assert!(
                !contexts_differ(source_path, dest_path),
                "Contexts should be the same after preserving"
            );
        } else {
            println!(
                "Note: Couldn't set SELinux context on source file to test preservation. This is expected if the test doesn't have sufficient permissions."
            );

            let preserve_result = preserve_security_context(source_path, dest_path);
            assert!(preserve_result.is_err());
        }

        let nonexistent_path = Path::new("/nonexistent/file/path");
        let result = preserve_security_context(nonexistent_path, dest_path);
        assert!(
            result.is_err(),
            "preserve_security_context should fail when source file doesn't exist"
        );

        let result = preserve_security_context(source_path, nonexistent_path);
        assert!(
            result.is_err(),
            "preserve_security_context should fail when destination file doesn't exist"
        );
    }

    #[test]
    fn test_preserve_security_context_empty_context() {
        let source_file = NamedTempFile::new().expect("Failed to create source tempfile");
        let dest_file = NamedTempFile::new().expect("Failed to create destination tempfile");
        let source_path = source_file.path();
        let dest_path = dest_file.path();

        if !is_selinux_enabled() {
            return;
        }

        let result = preserve_security_context(source_path, dest_path);

        if let Err(err) = result {
            match err {
                SeLinuxError::ContextSetFailure(_, _) => {
                    println!("Note: Could not set context due to permissions: {err}");
                }
                unexpected => {
                    panic!("Unexpected error: {unexpected}");
                }
            }
        }
    }
}
