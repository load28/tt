use std::fs;
use std::path::Path;

pub fn updating() -> bool {
    std::env::var_os("UPDATE_EXPECT").is_some()
}

fn regenerate() -> String {
    format!(
        "Run `UPDATE_EXPECT=1 cargo test --test {}` and review the diff.",
        env!("CARGO_CRATE_NAME")
    )
}

pub fn expect(path: &Path, actual: &str) {
    if updating() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("writable baseline directory");
        }
        fs::write(path, actual).expect("writable baseline");
        return;
    }
    let expected = fs::read_to_string(path).unwrap_or_else(|_| {
        panic!(
            "missing baseline: {} does not exist yet.\n{}",
            path.display(),
            regenerate()
        )
    });
    if expected == actual {
        return;
    }
    panic!(
        "modified baseline: {} is out of date\n\n{}\n{}",
        path.display(),
        diff(&expected, actual),
        regenerate()
    );
}

pub fn expect_absent(path: &Path) {
    if !path.exists() {
        return;
    }
    if updating() {
        fs::remove_file(path).expect("removable baseline");
        return;
    }
    panic!(
        "stale baseline: {} exists, but the run produced nothing for it.\n{}",
        path.display(),
        regenerate()
    );
}

pub fn diff(expected: &str, actual: &str) -> String {
    const CONTEXT: usize = 3;
    let expected: Vec<&str> = expected.lines().collect();
    let actual: Vec<&str> = actual.lines().collect();

    let head = expected
        .iter()
        .zip(&actual)
        .take_while(|(left, right)| left == right)
        .count();
    let tail = expected[head..]
        .iter()
        .rev()
        .zip(actual[head..].iter().rev())
        .take_while(|(left, right)| left == right)
        .count();

    let mut out = String::new();
    let from = head.saturating_sub(CONTEXT);
    if from > 0 {
        out.push_str(&format!("  ... {from} identical line(s)\n"));
    }
    for line in &expected[from..head] {
        out.push_str(&format!("  {line}\n"));
    }
    for line in &expected[head..expected.len() - tail] {
        out.push_str(&format!("- {line}\n"));
    }
    for line in &actual[head..actual.len() - tail] {
        out.push_str(&format!("+ {line}\n"));
    }
    let after = expected.len() - tail;
    let shown = tail.min(CONTEXT);
    for line in &expected[after..after + shown] {
        out.push_str(&format!("  {line}\n"));
    }
    if tail > shown {
        out.push_str(&format!("  ... {} identical line(s)\n", tail - shown));
    }
    out
}
