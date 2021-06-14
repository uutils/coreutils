pub mod completions {
    use super::get_app;
    use std::env;
    use std::path::Path;

    pub fn gen_completions() {
        let out_dir = env::var("OUT_DIR").unwrap();
        let module = env::var("CARGO_MANIFEST_DIR").unwrap();
        let prefix = env::var("PROG_PREFIX").unwrap_or_default();
        let executable = Path::new(&module).file_name().unwrap().to_str().unwrap();
        let mut app = get_app(executable);
        for shell in &clap::Shell::variants() {
            app.gen_completions(
                format!("{}{}", prefix, executable),
                shell.parse().unwrap(),
                &out_dir,
            );
        }
    }
}
