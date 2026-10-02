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

#[derive(Clone)]
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

pub fn strip_ranges(raw: &str, path: &Path) -> (String, Vec<(usize, usize)>) {
    let mut text = String::new();
    let mut ranges = Vec::new();
    let mut open = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        if raw[i..].starts_with("[|") {
            open.push(text.len());
            i += 2;
            continue;
        }
        if raw[i..].starts_with("|]") {
            let start = open
                .pop()
                .unwrap_or_else(|| panic!("{}: `|]` without `[|`", path.display()));
            ranges.push((start, text.len()));
            i += 2;
            continue;
        }
        let width = raw[i..].chars().next().map_or(1, char::len_utf8);
        text.push_str(&raw[i..i + width]);
        i += width;
    }
    assert!(open.is_empty(), "{}: `[|` without `|]`", path.display());
    ranges.sort_unstable();
    (text, ranges)
}

pub fn line_col(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset];
    let line = before.matches('\n').count() + 1;
    let col = before[before.rfind('\n').map_or(0, |i| i + 1)..]
        .chars()
        .count()
        + 1;
    (line, col)
}

pub fn example_blocks(explanation: &str) -> Vec<String> {
    let lines: Vec<&str> = explanation.lines().collect();
    let mut blocks: Vec<Vec<&str>> = Vec::new();
    let mut open = false;
    for (index, line) in lines.iter().enumerate() {
        if let Some(code) = line.strip_prefix("    ") {
            if !open {
                blocks.push(Vec::new());
                open = true;
            }
            blocks.last_mut().unwrap().push(code);
        } else if open
            && line.trim().is_empty()
            && lines
                .get(index + 1)
                .is_some_and(|next| next.starts_with("    "))
        {
            blocks.last_mut().unwrap().push("");
        } else {
            open = false;
        }
    }
    blocks.into_iter().map(|block| block.join("\n")).collect()
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
