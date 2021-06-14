include!("../base32/src/app.rs");
include!("../../build_completions.rs");
fn main() {
    completions::gen_completions();
}
