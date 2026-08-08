fn main() {
    // Set a short alias for the WASI-without-threads configuration so that
    // source files can use `#[cfg(wasi_no_threads)]`.
    println!("cargo::rustc-check-cfg=cfg(wasi_no_threads)");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target = std::env::var("TARGET").unwrap_or_default();

    if target_os == "wasi" && target != "wasm32-wasip1-threads" {
        println!("cargo::rustc-cfg=wasi_no_threads");
    }
}
