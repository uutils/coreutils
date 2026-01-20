
fn main() {
    let s = std::fs::read_to_string("Cargo.toml").unwrap();
    let v: Vec<_> = s.lines()
        .skip_while(|l| !l.contains("feat_require_unix_core = [")).skip(1)
        .take_while(|l| !l.contains(']'))
        .filter_map(|l| l.split('"').nth(1))
        .collect();
    println!("{}", v.join(" "));
}
