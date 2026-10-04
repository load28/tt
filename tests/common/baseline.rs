mod selection;
pub use selection::filtered_by;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub fn updating() -> bool {
    std::env::var_os("UPDATE_EXPECT").is_some()
}

fn regenerate() -> String {
    format!(
        "Run `scripts/baseline-diff` to review this run's new baselines and `scripts/baseline-accept` \
         to accept them (or `UPDATE_EXPECT=1 cargo test --test {}`).",
        env!("CARGO_CRATE_NAME")
    )
}

fn filtered() -> bool {
    filtered_by(
        std::env::args().skip(1),
        std::env::var("TT_CASES").ok().as_deref(),
    )
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

pub fn not_sampled(path: &Path) {
    record(path, "unsampled");
}

pub fn expect(path: &Path, actual: &str) {
    if let Err(message) = compare(path, actual) {
        panic!("{message}");
    }
}

pub fn expect_absent(path: &Path) {
    if let Err(message) = compare_absent(path) {
        panic!("{message}");
    }
}

pub fn finish(failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "{} baseline(s) differ:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

pub fn local_path(path: &Path) -> PathBuf {
    let reference = root().join("tests/baselines/reference");
    let local = root().join("tests/baselines/local");
    if let Ok(inside) = path.strip_prefix(&reference) {
        assert!(
            !inside.starts_with("tests"),
            "{}: a reference baseline under a `tests` directory would share its local path with a fixture's",
            path.display()
        );
        return local.join(inside);
    }
    local.join(path.strip_prefix(root()).unwrap_or(path))
}

fn marker(local: &Path) -> PathBuf {
    let mut name = local.as_os_str().to_owned();
    name.push(".delete");
    PathBuf::from(name)
}

fn clear_local(local: &Path) {
    let _ = fs::remove_file(local);
    let _ = fs::remove_file(marker(local));
}

fn write_local(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("writable local baseline directory");
    }
    fs::write(path, contents).expect("writable local baseline");
}

pub fn compare(path: &Path, actual: &str) -> Result<(), String> {
    record(path, "present");
    let local = local_path(path);
    clear_local(&local);
    if updating() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("writable baseline directory");
        }
        fs::write(path, actual).expect("writable baseline");
        return Ok(());
    }
    let Ok(expected) = fs::read_to_string(path) else {
        write_local(&local, actual);
        return Err(format!(
            "missing baseline: {} does not exist yet; the new one is at {}.\n{}",
            path.display(),
            local.display(),
            regenerate()
        ));
    };
    if expected == actual {
        return Ok(());
    }
    write_local(&local, actual);
    Err(format!(
        "modified baseline: {} is out of date; the new one is at {}\n\n{}\n{}",
        path.display(),
        local.display(),
        diff(&expected, actual),
        regenerate()
    ))
}

pub fn compare_absent(path: &Path) -> Result<(), String> {
    record(path, "absent");
    let local = local_path(path);
    clear_local(&local);
    if !path.exists() {
        return Ok(());
    }
    if updating() {
        fs::remove_file(path).expect("removable baseline");
        return Ok(());
    }
    write_local(&marker(&local), "");
    Err(format!(
        "stale baseline: {} exists, but the run produced nothing for it.\n{}",
        path.display(),
        regenerate()
    ))
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
