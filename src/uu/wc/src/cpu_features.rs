// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::env;
use std::sync::OnceLock;

#[derive(Debug)]
pub(crate) struct SimdPolicy {
    disabled_by_env: Vec<String>,
    available: Vec<&'static str>,
}

impl SimdPolicy {
    fn detect() -> Self {
        let tunables = env::var_os("GLIBC_TUNABLES")
            .and_then(|value| value.into_string().ok())
            .unwrap_or_default();

        let disabled_by_env = parse_disabled_features(&tunables);
        let available = detect_available_features();

        Self {
            disabled_by_env,
            available,
        }
    }

    pub(crate) fn env_allows_simd(&self) -> bool {
        self.disabled_by_env.is_empty()
    }

    pub(crate) fn disabled_features(&self) -> &[String] {
        &self.disabled_by_env
    }

    pub(crate) fn available_features(&self) -> &[&'static str] {
        &self.available
    }
}

static SIMD_POLICY: OnceLock<SimdPolicy> = OnceLock::new();

pub(crate) fn simd_policy() -> &'static SimdPolicy {
    SIMD_POLICY.get_or_init(SimdPolicy::detect)
}

fn parse_disabled_features(tunables: &str) -> Vec<String> {
    if tunables.is_empty() {
        return Vec::new();
    }

    let mut disabled = Vec::new();

    for entry in tunables.split(':') {
        let entry = entry.trim();
        let Some((name, raw_value)) = entry.split_once('=') else {
            continue;
        };

        if name.trim() != "glibc.cpu.hwcaps" {
            continue;
        }

        for token in raw_value.split(',') {
            let token = token.trim();
            if !token.starts_with('-') {
                continue;
            }
            let feature = token.trim_start_matches('-').to_ascii_uppercase();
            if !feature.is_empty() {
                disabled.push(feature);
            }
        }
    }

    disabled
}

fn detect_available_features() -> Vec<&'static str> {
    let mut features = Vec::new();
    #[cfg(any(target_arch = "x86", target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") {
            features.push("AVX2");
        }
        if std::arch::is_x86_feature_detected!("sse2") {
            features.push("SSE2");
        }
    }
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    {
        if std::arch::is_aarch64_feature_detected!("asimd") {
            features.push("ASIMD");
        }
    }
    features
}
