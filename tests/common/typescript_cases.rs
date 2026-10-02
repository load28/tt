use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

pub fn cases_required() -> bool {
    std::env::var_os("TTC_REQUIRE_TYPESCRIPT_CASES").is_some_and(|v| !v.is_empty() && v != "0")
}

pub fn manifest() -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/typescript-cases.json");
    let text = std::fs::read_to_string(&path).expect("tests/typescript-cases.json");
    serde_json::from_str(&text).expect("tests/typescript-cases.json is JSON")
}

pub fn cases_checkout(manifest: &serde_json::Value) -> Option<PathBuf> {
    if let Some(named) = std::env::var_os("TTC_TYPESCRIPT_CASES_DIR").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(named));
    }
    let commit = manifest["commit"]
        .as_str()
        .expect("the manifest names a commit");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/typescript-cases")
        .join(commit);
    dir.join(".git").is_dir().then_some(dir)
}

pub fn tree(manifest: &serde_json::Value, suffix: &str) -> String {
    manifest["trees"]
        .as_object()
        .expect("the manifest lists its trees")
        .keys()
        .find(|tree| tree.ends_with(suffix))
        .unwrap_or_else(|| panic!("the manifest has no tree ending in {suffix}"))
        .clone()
}

pub fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

pub fn seeded_choice(total: usize, count: usize, seed: u64) -> std::collections::BTreeSet<usize> {
    let count = count.min(total);
    let mut state = seed;
    let mut indices: Vec<usize> = (0..total).collect();
    for i in 0..count {
        let j = i + (splitmix(&mut state) % (total - i) as u64) as usize;
        indices.swap(i, j);
    }
    indices[..count].iter().copied().collect()
}

pub struct Oracle {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Oracle {
    pub fn start() -> Oracle {
        let api = super::typescript()
            .expect("the pinned TypeScript")
            .join("dist/api/sync/api.js");
        let mut child = Command::new("node")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/typescript-diagnostics.mjs"))
            .arg(api)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("node runs the TypeScript oracle");
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
        Oracle {
            child,
            stdin,
            stdout,
        }
    }

    pub fn ask(&mut self, request: &serde_json::Value) -> serde_json::Value {
        writeln!(self.stdin, "{request}").expect("the oracle reads");
        self.stdin.flush().expect("the oracle reads");
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .expect("the oracle answers");
        let answer: serde_json::Value = serde_json::from_str(&line)
            .unwrap_or_else(|e| panic!("the oracle answered no JSON ({e}): {line}"));
        assert!(
            answer.get("error").is_none(),
            "the oracle failed on {request}: {}",
            answer["error"]
        );
        answer
    }
}

impl Drop for Oracle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
