include!("src/app.rs");
include!("../../build_completions.rs");
fn main() {
    completions::gen_completions();
}
