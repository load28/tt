use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

pub fn updating() -> bool {
    std::env::var_os("UPDATE_EXPECT").is_some()
}

fn regenerate() -> String {
    format!(
        "Run `UPDATE_EXPECT=1 cargo test --test {}` and review the diff.",
        env!("CARGO_CRATE_NAME")
    )
}

fn filtered() -> bool {
    if std::env::var_os("TT_CASES").is_some_and(|value| !value.is_empty()) {
        return true;
    }
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--skip" | "--ignored" | "--list" => return true,
            "--test-threads" | "--color" | "--format" | "--logfile" | "--shuffle-seed" | "-Z" => {
                args.next();
            }
            _ if arg.starts_with('-') => {}
            _ => return true,
        }
    }
    false
}

fn record(path: &Path, state: &str) {
    let Some(dir) = std::env::var_os("TT_BASELINE_TRACKING_DIR").filter(|dir| !dir.is_empty())
    else {
        return;
    };
    static LOG: OnceLock<Mutex<fs::File>> = OnceLock::new();
    let log = LOG.get_or_init(|| {
        let dir = Path::new(&dir);
        fs::create_dir_all(dir).expect("a writable baseline tracking directory");
        let binary = env!("CARGO_CRATE_NAME");
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join(format!("{binary}-{}.txt", std::process::id())))
            .expect("a writable baseline tracking file");
        writeln!(file, "binary {binary} filtered {}", filtered()).expect("tracking header");
        Mutex::new(file)
    });
    let relative = path
        .strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    let mut file = log.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    writeln!(file, "{state} {relative}").expect("tracking entry");
}

pub fn expect(path: &Path, actual: &str) {
    record(path, "present");
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
    record(path, "absent");
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
