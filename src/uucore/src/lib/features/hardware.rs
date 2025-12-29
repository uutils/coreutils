// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! CPU hardware capability detection for performance-sensitive utilities
//!
//! This module provides a unified interface for detecting CPU features and
//! respecting environment-based SIMD policies (e.g., GLIBC_TUNABLES).
//!
//! It provides 2 structures, from which we can get capabilities:
//! - [`CpuFeatures`], which contains the raw available CPU features;
//! - [`SimdPolicy`], which relies on [`CpuFeatures`] and the `GLIBC_TUNABLES`
//!   environment variable to get the *enabled* CPU features
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
//! use uucore::hardware::{CpuFeatures, SimdPolicy, HasHardwareFeatures as _};
//!
//! // Simple hardware detection
//! let features = CpuFeatures::detect();
//! if features.has_avx2() {
//!     println!("CPU has AVX2 support");
//! }
//!
//! // Check SIMD policy (respects GLIBC_TUNABLES)
//! let policy = SimdPolicy::detect();
//! if policy.has_avx2() {
//!     println!("CPU has AVX2 support and it is not disabled by env");
//! }
//! if policy.allows_simd() {
//!     // Use SIMD-accelerated path
//! } else {
//!     // Fall back to scalar implementation
//! }
//! ```

use std::collections::BTreeSet;
use std::env;
use std::sync::OnceLock;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum HardwareFeature {
    /// AVX-512 support (x86/x86_64 only)
    Avx512,
    /// AVX2 support (x86/x86_64 only)
    Avx2,
    /// PCLMULQDQ support for CRC acceleration (x86/x86_64 only)
    PclMul,
    /// VMULL support for CRC acceleration (ARM only)
    Vmull,
    /// SSE2 support (x86/x86_64 only)
    Sse2,
    /// ARM ASIMD/NEON support (aarch64 only)
    Asimd,
}

pub struct InvalidHardwareFeature;

impl TryFrom<&str> for HardwareFeature {
    type Error = InvalidHardwareFeature;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        use HardwareFeature::*;
        match value {
            "AVX512" | "AVX512F" => Ok(Avx512),
            "AVX2" => Ok(Avx2),
            "PCLMUL" | "PMULL" => Ok(PclMul),
            "VMULL" => Ok(Vmull),
            "SSE2" => Ok(Sse2),
            "ASIMD" => Ok(Asimd),
            _ => Err(InvalidHardwareFeature),
        }
    }
}

/// Trait for implementing common hardware feature checks.
///
/// This is used for the `CpuFeatures` struct, that holds the CPU capabilities,
/// and for the `SimdPolicy` type that computes the enabled features with the
/// environment variables.
pub trait HasHardwareFeatures {
    fn has_feature(&self, feat: HardwareFeature) -> bool;

    fn iter_features(&self) -> impl Iterator<Item = HardwareFeature>;

    /// Check if AVX-512 is available (x86/x86_64 only)
    #[inline]
    fn has_avx512(&self) -> bool {
        self.has_feature(HardwareFeature::Avx512)
    }

    /// Check if AVX2 is available (x86/x86_64 only)
    #[inline]
    fn has_avx2(&self) -> bool {
        self.has_feature(HardwareFeature::Avx2)
    }

    /// Check if PCLMULQDQ is available (x86/x86_64 only)
    #[inline]
    fn has_pclmul(&self) -> bool {
        self.has_feature(HardwareFeature::PclMul)
    }

    /// Check if VMULL is available (ARM only)
    #[inline]
    fn has_vmull(&self) -> bool {
        self.has_feature(HardwareFeature::Vmull)
    }

    /// Check if SSE2 is available (x86/x86_64 only)
    #[inline]
    fn has_sse2(&self) -> bool {
        self.has_feature(HardwareFeature::Sse2)
    }

    /// Check if ARM ASIMD/NEON is available (aarch64 only)
    #[inline]
    fn has_asimd(&self) -> bool {
        self.has_feature(HardwareFeature::Asimd)
    }
}

/// CPU hardware features that affect performance
///
/// Provides platform-specific CPU feature detection with caching.
/// Detection is performed once and cached for the lifetime of the process.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CpuFeatures {
    set: BTreeSet<HardwareFeature>,
}
impl CpuFeatures {
    pub fn detect() -> &'static Self {
        static FEATURES: OnceLock<CpuFeatures> = OnceLock::new();
        FEATURES.get_or_init(Self::detect_impl)
    }

    fn detect_impl() -> Self {
        let set = [
            (HardwareFeature::Avx512, detect_avx512 as fn() -> bool),
            (HardwareFeature::Avx2, detect_avx2),
            (HardwareFeature::PclMul, detect_pclmul),
            (HardwareFeature::Vmull, detect_vmull),
            (HardwareFeature::Sse2, detect_sse2),
            (HardwareFeature::Asimd, detect_asimd),
        ]
        .into_iter()
        .filter_map(|(feat, detect)| detect().then_some(feat))
        .collect();

        Self { set }
    }
}

impl HasHardwareFeatures for CpuFeatures {
    fn has_feature(&self, feat: HardwareFeature) -> bool {
        self.set.contains(&feat)
    }

    fn iter_features(&self) -> impl Iterator<Item = HardwareFeature> {
        self.set.iter().copied()
    }
}

/// SIMD policy based on environment variables
///
/// Respects GLIBC_TUNABLES environment variable to disable specific CPU features.
/// This is used by GNU utilities to allow users to disable hardware acceleration.
#[derive(Debug, Clone)]
pub struct SimdPolicy {
    /// Features disabled via GLIBC_TUNABLES (e.g., ["AVX2", "AVX512F"])
    disabled_by_env: BTreeSet<HardwareFeature>,
    hardware_features: &'static CpuFeatures,
}

impl SimdPolicy {
    /// Get the global SIMD policy (cached)
    ///
    /// This checks both hardware capabilities and the GLIBC_TUNABLES environment
    /// variable. The result is cached for the lifetime of the process.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use uucore::hardware::SimdPolicy;
    ///
    /// let policy = SimdPolicy::detect();
    /// if policy.allows_simd() {
    ///     println!("SIMD is enabled");
    /// } else {
    ///     println!("SIMD disabled by: {:?}", policy.disabled_features());
    /// }
    /// ```
    pub fn detect() -> &'static Self {
        static POLICY: OnceLock<SimdPolicy> = OnceLock::new();
        POLICY.get_or_init(Self::detect_impl)
    }

    fn detect_impl() -> Self {
        let tunables = env::var("GLIBC_TUNABLES").unwrap_or_default();
        let disabled_by_env = parse_disabled_features(&tunables);
        let hardware_features = CpuFeatures::detect();

        Self {
            disabled_by_env,
            hardware_features,
        }
    }

    /// Returns true if any SIMD feature remains enabled after applying GLIBC_TUNABLES.
    pub fn allows_simd(&self) -> bool {
        self.iter_features().next().is_some()
    }

    pub fn disabled_features(&self) -> Vec<HardwareFeature> {
        self.disabled_by_env.iter().copied().collect()
    }
}

impl HasHardwareFeatures for SimdPolicy {
    fn has_feature(&self, feat: HardwareFeature) -> bool {
        self.hardware_features.has_feature(feat) && !self.disabled_by_env.contains(&feat)
    }

    fn iter_features(&self) -> impl Iterator<Item = HardwareFeature> {
        self.hardware_features
            .set
            .difference(&self.disabled_by_env)
            .copied()
    }
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
fn parse_disabled_features(tunables: &str) -> BTreeSet<HardwareFeature> {
    if tunables.is_empty() {
        return BTreeSet::new();
    }

    let mut disabled = BTreeSet::new();

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
                let feature =
                    HardwareFeature::try_from(feature.trim().to_ascii_uppercase().as_str());
                if let Ok(feature) = feature {
                    disabled.insert(feature);
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
    fn test_parse_disabled_features_empty() {
        assert_eq!(parse_disabled_features(""), BTreeSet::new());
    }

    #[test]
    fn test_parse_disabled_features_single() {
        let result = parse_disabled_features("glibc.cpu.hwcaps=-AVX2");
        let mut expected = BTreeSet::new();

        expected.insert(HardwareFeature::Avx2);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_disabled_features_multiple() {
        let result = parse_disabled_features("glibc.cpu.hwcaps=-AVX2,-AVX512F");
        let mut expected = BTreeSet::new();

        expected.insert(HardwareFeature::Avx2);
        expected.insert(HardwareFeature::Avx512);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_disabled_features_mixed() {
        let result = parse_disabled_features("glibc.cpu.hwcaps=-AVX2,SSE2,-AVX512F");
        let mut expected = BTreeSet::new();

        expected.insert(HardwareFeature::Avx2);
        expected.insert(HardwareFeature::Avx512);

        // Only features with '-' prefix are disabled
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_disabled_features_with_other_tunables() {
        let result =
            parse_disabled_features("glibc.malloc.check=1:glibc.cpu.hwcaps=-AVX2:other=value");
        let mut expected = BTreeSet::new();

        expected.insert(HardwareFeature::Avx2);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_disabled_features_case_insensitive() {
        let result = parse_disabled_features("glibc.cpu.hwcaps=-avx2,-Avx512f");
        let mut expected = BTreeSet::new();

        expected.insert(HardwareFeature::Avx2);
        expected.insert(HardwareFeature::Avx512);

        // Only features with '-' prefix are disabled
        assert_eq!(result, expected);
    }

    #[test]
    fn test_simd_policy() {
        let policy = SimdPolicy::detect();
        // Just verify it works
        let _ = policy.allows_simd();
    }

    #[test]
    fn test_simd_policy_caching() {
        let policy1 = SimdPolicy::detect();
        let policy2 = SimdPolicy::detect();
        // Should be same instance (pointer equality)
        assert!(std::ptr::eq(policy1, policy2));
    }
}
