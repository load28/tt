use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_TSCONFIG: &str = r#"{
  "compilerOptions": {
    "target": "es2022",
    "module": "preserve",
    "moduleResolution": "bundler",
    "jsx": "preserve",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true
  }
}
"#;

pub struct Unit {
    pub name: String,
    pub content: String,
}

pub struct Parsed {
    pub units: Vec<Unit>,
    pub directives: Vec<(String, String)>,
}

pub fn directive(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("//")?.trim_start().strip_prefix('@')?;
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    let (name, tail) = rest.split_at(end);
    let value = tail.trim_start().strip_prefix(':')?;
    Some((name, value.trim()))
}

pub fn parse(text: &str, file_name: &str, path: &Path) -> Parsed {
    let mut directives = Vec::new();
    let mut units = Vec::new();
    let mut current: Option<(String, Vec<&str>)> = None;
    let mut preamble: Vec<&str> = Vec::new();
    for line in text.split('\n') {
        let Some((name, value)) = directive(line) else {
            match &mut current {
                Some((_, lines)) => lines.push(line),
                None => preamble.push(line),
            }
            continue;
        };
        if name.eq_ignore_ascii_case("filename") {
            if let Some((name, lines)) = current.take() {
                units.push(Unit {
                    name,
                    content: lines.join("\n"),
                });
            } else {
                assert!(
                    preamble.iter().all(|l| {
                        let l = l.trim();
                        l.is_empty() || l.starts_with("//")
                    }),
                    "{}: non-comment content appears before the first `// @filename`",
                    path.display()
                );
            }
            current = Some((value.to_string(), Vec::new()));
        } else {
            directives.push((name.to_ascii_lowercase(), value.to_string()));
        }
    }
    match current {
        Some((name, lines)) => units.push(Unit {
            name,
            content: lines.join("\n"),
        }),
        None => units.push(Unit {
            name: file_name.to_string(),
            content: preamble.join("\n"),
        }),
    }
    Parsed { units, directives }
}

pub fn is_tt(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("tt" | "ttx")
    )
}

pub fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.expect("readable case entry").path();
        if path.is_dir() {
            files(&path, out);
        } else if is_tt(&path) {
            out.push(path);
        }
    }
}
