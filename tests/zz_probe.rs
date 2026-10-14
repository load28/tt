#[test]
fn probe() {
    use ttc::*;
    let path = std::env::var("PROBE").unwrap();
    let s = std::fs::read_to_string(&path).unwrap();
    let options = Options { defer_to_checker: true, rewrite_imports: ImportRewrite::Off, ..Options::default() };
    let b = s.as_bytes();
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut i = 0;
    std::panic::set_hook(Box::new(|_| {}));
    while i < b.len() {
        if word(b[i]) && (i == 0 || !word(b[i - 1])) {
            let mut e = i; while e < b.len() && word(b[e]) { e += 1; }
            for rep in ["", "_", "_ => 0, "] {
                let t = format!("{}{}{}", &s[..i], rep, &s[e..]);
                if std::panic::catch_unwind(|| { let _ = compile_projection_report(&t, &options); }).is_err() {
                    println!("PANIC removing {:?} with {:?} at {}: {:?}", &s[i..e], rep, i, &t[i.saturating_sub(40)..(i+40).min(t.len())]);
                }
            }
            i = e;
        } else { i += 1; }
    }
}
