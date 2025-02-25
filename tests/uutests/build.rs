use cargo_metadata::MetadataCommand;
use std::env;

fn main() {
    // Get the target directory from cargo
    let metadata = MetadataCommand::new().no_deps().exec().unwrap();
    let target_dir = metadata.target_directory;

    // Determine the profile (debug or release)
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    // Get target and host triples
    let target_triple = env::var("TARGET");
    let host_triple = env::var("HOST");

    // Debug: Print the values we're working with
    println!("cargo:warning=Target triple: {:?}", target_triple);
    println!("cargo:warning=Host triple: {:?}", host_triple);

    // Construct the path to the coreutils binary
    let binary_name = if cfg!(windows) {
        "coreutils.exe"
    } else {
        "coreutils"
    };

    // Try both possible paths
    let direct_path = target_dir.join(&profile).join(binary_name);
    let target_path = if let Ok(target) = target_triple {
        target_dir.join(&target).join(&profile).join(binary_name)
    } else {
        direct_path.clone()
    };

    // Check which path exists
    let binary_path = if target_path.exists() {
        target_path
    } else {
        direct_path
    };

    println!("cargo:warning=Checking path: {}", binary_path);

    // Output the binary path for use in the tests
    println!("cargo:rustc-env=CARGO_BIN_EXE_coreutils={}", binary_path);
}
