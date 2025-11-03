// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! CPU hardware capability detection for cksum --debug
//!
//! This module detects available CPU features that affect cksum performance,
//! matching GNU cksum's --debug behavior.

use std::sync::Once;

/// CPU features that affect cksum performance
#[derive(Debug, Clone, Copy)]
pub struct CpuFeatures {
    pub avx512: bool,
    pub avx2: bool,
    pub pclmul: bool,
    pub vmull: bool,
}

impl CpuFeatures {
    /// Detect available CPU features (cached after first call)
    pub fn detect() -> Self {
        static ONCE: Once = Once::new();
        static mut FEATURES: CpuFeatures = CpuFeatures {
            avx512: false,
            avx2: false,
            pclmul: false,
            vmull: false,
        };

        unsafe {
            ONCE.call_once(|| {
                FEATURES = Self {
                    avx512: has_avx512(),
                    avx2: has_avx2(),
                    pclmul: has_pclmul(),
                    vmull: has_vmull(),
                };
            });
            FEATURES
        }
    }

    /// Print debug information to stderr
    /// Outputs CPU feature availability in GNU cksum format
    pub fn print_debug(&self) {
        self.print_feature("avx512", self.avx512);
        self.print_feature("avx2", self.avx2);
        self.print_feature("pclmul", self.pclmul);
        if cfg!(target_arch = "aarch64") {
            self.print_feature("vmull", self.vmull);
        }
    }

    fn print_feature(&self, name: &str, available: bool) {
        let status = if available {
            format!("using {name} hardware support")
        } else {
            format!("{name} support not detected")
        };
        eprintln!("cksum: {status}");
    }
}

// CPU feature detection functions
// These use cpufeatures crate for cross-platform detection

#[cfg(all(
    any(target_arch = "x86_64", target_arch = "x86"),
    not(target_os = "android")
))]
fn has_avx512() -> bool {
    cpufeatures::new!(cpuid_avx512, "avx512f", "avx512bw");
    cpuid_avx512::get()
}

#[cfg(not(all(
    any(target_arch = "x86_64", target_arch = "x86"),
    not(target_os = "android")
)))]
fn has_avx512() -> bool {
    false
}

#[cfg(all(
    any(target_arch = "x86_64", target_arch = "x86"),
    not(target_os = "android")
))]
fn has_avx2() -> bool {
    cpufeatures::new!(cpuid_avx2, "avx2");
    cpuid_avx2::get()
}

#[cfg(not(all(
    any(target_arch = "x86_64", target_arch = "x86"),
    not(target_os = "android")
)))]
fn has_avx2() -> bool {
    false
}

#[cfg(all(
    any(target_arch = "x86_64", target_arch = "x86"),
    not(target_os = "android")
))]
fn has_pclmul() -> bool {
    cpufeatures::new!(cpuid_pclmul, "pclmulqdq");
    cpuid_pclmul::get()
}

#[cfg(not(all(
    any(target_arch = "x86_64", target_arch = "x86"),
    not(target_os = "android")
)))]
fn has_pclmul() -> bool {
    false
}

#[cfg(target_arch = "aarch64")]
fn has_vmull() -> bool {
    // ARM NEON support detection
    // This would require platform-specific code
    // For now, return false as a safe default
    false
}

#[cfg(not(target_arch = "aarch64"))]
fn has_vmull() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(target_os = "android"))]
    fn test_cpu_features_detection() {
        let features = CpuFeatures::detect();
        // Features should be valid booleans - just verify they can be detected
        let _ = features.avx512;
        let _ = features.avx2;
        let _ = features.pclmul;
        let _ = features.vmull;
    }

    #[test]
    #[cfg(not(target_os = "android"))]
    fn test_cpu_features_cached() {
        let features1 = CpuFeatures::detect();
        let features2 = CpuFeatures::detect();
        // Should return same values (cached)
        assert_eq!(features1.avx512, features2.avx512);
        assert_eq!(features1.avx2, features2.avx2);
        assert_eq!(features1.pclmul, features2.pclmul);
        assert_eq!(features1.vmull, features2.vmull);
    }

    #[test]
    #[cfg(all(
        any(target_arch = "x86_64", target_arch = "x86"),
        not(target_os = "android")
    ))]
    fn test_cpu_features_on_x86() {
        let features = CpuFeatures::detect();
        // On x86/x86_64, at least one feature should be detected or all false
        // (depending on CPU capabilities)
        let _ = features;
    }
}
