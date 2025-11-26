// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! CPU hardware capability detection for performance-sensitive utilities
//!
//! This module provides a unified interface for detecting CPU features and
//! respecting environment-based SIMD policies (e.g., GLIBC_TUNABLES).
//!
//! # Use Cases
//!
//! - `cksum --debug`: Report hardware acceleration capabilities
//! - `wc --debug`: Report SIMD usage and GLIBC_TUNABLES restrictions
//! - Runtime decisions: Enable/disable SIMD paths based on environment
//!
//! # Examples
//!
//! ```no_run
//! use uucore::hardware::{CpuFeatures, simd_policy};
//!
//! // Simple hardware detection
//! let features = CpuFeatures::detect();
//! if features.has_avx2() {
//!     println!("AVX2 is available");
//! }
//!
//! // Check SIMD policy (respects GLIBC_TUNABLES)
//! let policy = simd_policy();
//! if policy.allows_simd() {
//!     // Use SIMD-accelerated path
//! } else {
//!     // Fall back to scalar implementation
//! }
//! ```

use std::env;
use std::sync::OnceLock;

/// CPU hardware features that affect performance
///
/// Provides platform-specific CPU feature detection with caching.
/// Detection is performed once and cached for the lifetime of the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuFeatures {
    /// AVX-512 support (x86/x86_64 only)
    avx512: bool,
    /// AVX2 support (x86/x86_64 only)
    avx2: bool,
    /// PCLMULQDQ support for CRC acceleration (x86/x86_64 only)
    pclmul: bool,
    /// VMULL support for CRC acceleration (ARM only)
    vmull: bool,
    /// SSE2 support (x86/x86_64 only)
    sse2: bool,
    /// ARM ASIMD/NEON support (aarch64 only)
    asimd: bool,
}

impl CpuFeatures {
    /// Detect available CPU features (cached after first call)
    ///
    /// This function uses a singleton pattern to ensure feature detection
    /// happens only once per process. Thread-safe.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use uucore::hardware::CpuFeatures;
    ///
    /// let features = CpuFeatures::detect();
    /// println!("AVX2: {}", features.has_avx2());
    /// ```
    pub fn detect() -> Self {
        static FEATURES: OnceLock<CpuFeatures> = OnceLock::new();
        *FEATURES.get_or_init(Self::detect_impl)
    }

    fn detect_impl() -> Self {
        Self {
            avx512: detect_avx512(),
            avx2: detect_avx2(),
            pclmul: detect_pclmul(),
            vmull: detect_vmull(),
            sse2: detect_sse2(),
            asimd: detect_asimd(),
        }
    }

    /// Check if AVX-512 is available (x86/x86_64 only)
    pub fn has_avx512(&self) -> bool {
        self.avx512
    }

    /// Check if AVX2 is available (x86/x86_64 only)
    pub fn has_avx2(&self) -> bool {
        self.avx2
    }

    /// Check if PCLMULQDQ is available (x86/x86_64 only)
    pub fn has_pclmul(&self) -> bool {
        self.pclmul
    }

    /// Check if VMULL is available (ARM only)
    pub fn has_vmull(&self) -> bool {
        self.vmull
    }

    /// Check if SSE2 is available (x86/x86_64 only)
    pub fn has_sse2(&self) -> bool {
        self.sse2
    }

    /// Check if ARM ASIMD/NEON is available (aarch64 only)
    pub fn has_asimd(&self) -> bool {
        self.asimd
    }

    /// Get list of available features as strings
    ///
    /// Returns uppercase feature names (e.g., "AVX2", "SSE2", "ASIMD")
    pub fn available_features(&self) -> Vec<&'static str> {
        let mut features = Vec::new();
        if self.avx512 {
            features.push("AVX512");
        }
        if self.avx2 {
            features.push("AVX2");
        }
        if self.pclmul {
            features.push("PCLMUL");
        }
        if self.vmull {
            features.push("VMULL");
        }
        if self.sse2 {
            features.push("SSE2");
        }
        if self.asimd {
            features.push("ASIMD");
        }
        features
    }
}

/// SIMD policy based on environment variables
///
/// Respects GLIBC_TUNABLES environment variable to disable specific CPU features.
/// This is used by GNU utilities to allow users to disable hardware acceleration.
#[derive(Debug, Clone)]
pub struct SimdPolicy {
    /// Features disabled via GLIBC_TUNABLES (e.g., ["AVX2", "AVX512F"])
    disabled_by_env: Vec<String>,
    /// Hardware features actually available
    hardware_features: CpuFeatures,
}

impl SimdPolicy {
    /// Create a new SIMD policy by checking environment and hardware
    fn new() -> Self {
        let tunables = env::var("GLIBC_TUNABLES").unwrap_or_default();
        let disabled_by_env = parse_disabled_features(&tunables);
        let hardware_features = CpuFeatures::detect();

        Self {
            disabled_by_env,
            hardware_features,
        }
    }

    /// Check if SIMD operations are allowed
    ///
    /// Returns `false` if any features are disabled via GLIBC_TUNABLES,
    /// regardless of what's available in hardware.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use uucore::hardware::simd_policy;
    ///
    /// let policy = simd_policy();
    /// if policy.allows_simd() {
    ///     // Use SIMD-accelerated bytecount
    /// } else {
    ///     // Use scalar fallback
    /// }
    /// ```
    pub fn allows_simd(&self) -> bool {
        self.disabled_by_env.is_empty()
    }

    /// Get list of features disabled by environment
    pub fn disabled_features(&self) -> &[String] {
        &self.disabled_by_env
    }

    /// Get available hardware features
    pub fn hardware_features(&self) -> &CpuFeatures {
        &self.hardware_features
    }

    /// Get list of features that are both available and not disabled
    pub fn enabled_features(&self) -> Vec<&'static str> {
        if !self.allows_simd() {
            return Vec::new();
        }
        self.hardware_features.available_features()
    }
}

/// Get the global SIMD policy (cached)
///
/// This checks both hardware capabilities and the GLIBC_TUNABLES environment
/// variable. The result is cached for the lifetime of the process.
///
/// # Examples
///
/// ```no_run
/// use uucore::hardware::simd_policy;
///
/// let policy = simd_policy();
/// if policy.allows_simd() {
///     println!("SIMD is enabled");
/// } else {
///     println!("SIMD disabled by: {:?}", policy.disabled_features());
/// }
/// ```
pub fn simd_policy() -> &'static SimdPolicy {
    static POLICY: OnceLock<SimdPolicy> = OnceLock::new();
    POLICY.get_or_init(SimdPolicy::new)
}

// Platform-specific feature detection

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn detect_avx512() -> bool {
    if cfg!(target_os = "android") {
        false
    } else {
        std::arch::is_x86_feature_detected!("avx512f")
            && std::arch::is_x86_feature_detected!("avx512bw")
    }
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn detect_avx512() -> bool {
    false
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn detect_avx2() -> bool {
    if cfg!(target_os = "android") {
        false
    } else {
        std::arch::is_x86_feature_detected!("avx2")
    }
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn detect_avx2() -> bool {
    false
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn detect_pclmul() -> bool {
    if cfg!(target_os = "android") {
        false
    } else {
        std::arch::is_x86_feature_detected!("pclmulqdq")
    }
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn detect_pclmul() -> bool {
    false
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn detect_sse2() -> bool {
    if cfg!(target_os = "android") {
        false
    } else {
        std::arch::is_x86_feature_detected!("sse2")
    }
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn detect_sse2() -> bool {
    false
}

#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
fn detect_asimd() -> bool {
    if cfg!(target_os = "android") {
        false
    } else {
        std::arch::is_aarch64_feature_detected!("asimd")
    }
}

#[cfg(not(all(target_arch = "aarch64", target_endian = "little")))]
fn detect_asimd() -> bool {
    false
}

#[cfg(target_arch = "aarch64")]
fn detect_vmull() -> bool {
    // VMULL is part of ARM NEON/ASIMD
    // For now, we use ASIMD as a proxy
    detect_asimd()
}

#[cfg(not(target_arch = "aarch64"))]
fn detect_vmull() -> bool {
    false
}

// GLIBC_TUNABLES parsing

/// Parse GLIBC_TUNABLES environment variable for disabled features
///
/// Format: `glibc.cpu.hwcaps=-AVX2,-AVX512F`
/// Multiple tunable sections can be separated by colons.
fn parse_disabled_features(tunables: &str) -> Vec<String> {
    if tunables.is_empty() {
        return Vec::new();
    }

    let mut disabled = Vec::new();

    // GLIBC_TUNABLES format: "tunable1=value1:tunable2=value2"
    for entry in tunables.split(':') {
        let entry = entry.trim();
        let Some((name, raw_value)) = entry.split_once('=') else {
            continue;
        };

        // We only care about glibc.cpu.hwcaps
        if name.trim() != "glibc.cpu.hwcaps" {
            continue;
        }

        // Parse comma-separated features, disabled ones start with '-'
        for token in raw_value.split(',') {
            let token = token.trim();
            if let Some(feature) = token.strip_prefix('-') {
                let feature = feature.trim().to_ascii_uppercase();
                if !feature.is_empty() {
                    disabled.push(feature);
                }
            }
        }
    }

    disabled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_features_detection() {
        let features = CpuFeatures::detect();
        // Just verify it doesn't panic and returns consistent results
        let features2 = CpuFeatures::detect();
        assert_eq!(features, features2);
    }

    #[test]
    fn test_available_features() {
        let features = CpuFeatures::detect();
        let available = features.available_features();
        // Should return a list (may be empty on some platforms)
        assert!(available.iter().all(|s| !s.is_empty()));
    }

    #[test]
    fn test_parse_disabled_features_empty() {
        assert_eq!(parse_disabled_features(""), Vec::<String>::new());
    }

    #[test]
    fn test_parse_disabled_features_single() {
        let result = parse_disabled_features("glibc.cpu.hwcaps=-AVX2");
        assert_eq!(result, vec!["AVX2"]);
    }

    #[test]
    fn test_parse_disabled_features_multiple() {
        let result = parse_disabled_features("glibc.cpu.hwcaps=-AVX2,-AVX512F");
        assert_eq!(result, vec!["AVX2", "AVX512F"]);
    }

    #[test]
    fn test_parse_disabled_features_mixed() {
        let result = parse_disabled_features("glibc.cpu.hwcaps=-AVX2,SSE2,-AVX512F");
        // Only features with '-' prefix are disabled
        assert_eq!(result, vec!["AVX2", "AVX512F"]);
    }

    #[test]
    fn test_parse_disabled_features_with_other_tunables() {
        let result =
            parse_disabled_features("glibc.malloc.check=1:glibc.cpu.hwcaps=-AVX2:other=value");
        assert_eq!(result, vec!["AVX2"]);
    }

    #[test]
    fn test_parse_disabled_features_case_insensitive() {
        let result = parse_disabled_features("glibc.cpu.hwcaps=-avx2,-Avx512f");
        // Should normalize to uppercase
        assert_eq!(result, vec!["AVX2", "AVX512F"]);
    }

    #[test]
    fn test_simd_policy() {
        let policy = simd_policy();
        // Just verify it works
        let _ = policy.allows_simd();
        let _ = policy.disabled_features();
        let _ = policy.enabled_features();
    }

    #[test]
    fn test_simd_policy_caching() {
        let policy1 = simd_policy();
        let policy2 = simd_policy();
        // Should be same instance (pointer equality)
        assert!(std::ptr::eq(policy1, policy2));
    }
}
