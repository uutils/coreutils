fn main() {
    // Set a short alias for the WASI-without-threads configuration so that
    // source files can use `#[cfg(wasi_no_threads)]` instead of the verbose
    // `#[cfg(all(target_os = "wasi", not(target_feature = "atomics")))]`.
    println!("cargo::rustc-check-cfg=cfg(wasi_no_threads)");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let has_atomics = std::env::var("CARGO_CFG_TARGET_FEATURE")
        .is_ok_and(|f| f.split(',').any(|feat| feat == "atomics"));

    if target_os == "wasi" && !has_atomics {
        println!("cargo::rustc-cfg=wasi_no_threads");
    }
}
