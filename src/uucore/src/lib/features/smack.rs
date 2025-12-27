// This file is part of the uutils uucore package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore smackfs
//! SMACK (Simplified Mandatory Access Control Kernel) support

use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

use thiserror::Error;

use crate::error::{UError, strip_errno};
use crate::translate;

#[derive(Debug, Error)]
pub enum SmackError {
    #[error("{}", translate!("smack-error-not-enabled"))]
    SmackNotEnabled,

    #[error("{}", translate!("smack-error-label-retrieval-failure", "error" => strip_errno(.0)))]
    LabelRetrievalFailure(io::Error),

    #[error("{}", translate!("smack-error-label-set-failure", "context" => .0.clone(), "error" => strip_errno(.1)))]
    LabelSetFailure(String, io::Error),

    #[error("{}", translate!("smack-error-io", "error" => strip_errno(.0)))]
    IoError(#[from] io::Error),
}

impl UError for SmackError {
    fn code(&self) -> i32 {
        match self {
            Self::SmackNotEnabled => 1,
            Self::LabelRetrievalFailure(_) => 2,
            Self::LabelSetFailure(_, _) => 3,
            Self::IoError(_) => 4,
        }
    }
}

impl From<SmackError> for i32 {
    fn from(error: SmackError) -> Self {
        error.code()
    }
}

/// Checks if SMACK is enabled by verifying smackfs is mounted.
pub fn is_smack_enabled() -> bool {
    Path::new("/sys/fs/smackfs").exists()
}

/// Gets the SMACK label for the current process.
pub fn get_smack_label_for_self() -> Result<String, SmackError> {
    if !is_smack_enabled() {
        return Err(SmackError::SmackNotEnabled);
    }

    let mut label = String::new();
    fs::File::open("/proc/self/attr/current")
        .map_err(SmackError::LabelRetrievalFailure)?
        .read_to_string(&mut label)
        .map_err(SmackError::LabelRetrievalFailure)?;

    Ok(label.trim().to_string())
}

/// Sets the SMACK label for the current process.
pub fn set_smack_label_for_self(label: &str) -> Result<(), SmackError> {
    if !is_smack_enabled() {
        return Err(SmackError::SmackNotEnabled);
    }

    let label_owned = label.to_string();
    fs::File::create("/proc/self/attr/current")
        .map_err(|e| SmackError::LabelSetFailure(label_owned.clone(), e))?
        .write_all(label.as_bytes())
        .map_err(|e| SmackError::LabelSetFailure(label_owned, e))?;

    Ok(())
}

/// Gets the SMACK label for a filesystem path via xattr.
#[cfg(feature = "xattr")]
pub fn get_smack_label_for_path(path: &Path) -> Result<String, SmackError> {
    if !is_smack_enabled() {
        return Err(SmackError::SmackNotEnabled);
    }

    match xattr::get(path, "security.SMACK64") {
        Ok(Some(value)) => Ok(String::from_utf8_lossy(&value).trim().to_string()),
        Ok(None) => Err(SmackError::LabelRetrievalFailure(io::Error::new(
            io::ErrorKind::NotFound,
            "no SMACK label set",
        ))),
        Err(e) => Err(SmackError::LabelRetrievalFailure(e)),
    }
}

/// Sets the SMACK label for a filesystem path via xattr.
#[cfg(feature = "xattr")]
pub fn set_smack_label_for_path(path: &Path, label: &str) -> Result<(), SmackError> {
    if !is_smack_enabled() {
        return Err(SmackError::SmackNotEnabled);
    }

    xattr::set(path, "security.SMACK64", label.as_bytes())
        .map_err(|e| SmackError::LabelSetFailure(label.to_string(), e))
}
