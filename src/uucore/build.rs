// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) tzfile zoneinfo

use std::env;

pub fn main() {
    // If custom-tz-fmt feature is enabled, set an "embed_tz" config to decide whether
    // to embed a full timezone database, or we can just use `tzfile` (which reads
    // from /usr/share/zoneinfo).
    println!("cargo::rustc-check-cfg=cfg(embed_tz)");
    let custom_tz_fmt = env::var("CARGO_FEATURE_CUSTOM_TZ_FMT");
    if custom_tz_fmt.is_ok() {
        // TODO: It might be worth considering making this an option:
        //  - People concerned with executable size may be willing to forgo timezone database
        //    completely.
        //  - Some other people may want to use an embedded timezone database _anyway_, instead
        //    of the one provided by the system.
        if cfg!(windows) || cfg!(target_os = "android") {
            println!("cargo::rustc-cfg=embed_tz");
        }
    }
}
