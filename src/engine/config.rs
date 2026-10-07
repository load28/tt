use std::path::{Path, PathBuf};

/// Whether the project `files` belong to compiles JSX with `"jsx":
/// "preserve"`, the one setting under which TypeScript names the
/// JavaScript it emits for a `.tsx` file `.jsx` rather than `.js`
/// (`GetOutputExtension` in typescript-go's `internal/outputpaths`). The
/// project is the named `tsconfig`, or else the nearest `tsconfig.json` at
/// or above the files, as a project is opened; without one, TypeScript's
/// default leaves `jsx` unset. The option's value is read as TypeScript
/// reads an enumerated option, without regard to case.
///
/// # Errors
///
/// A configuration that cannot be read, is not JSON with comments, or
/// `extends` a configuration that cannot be found or extends itself.
pub fn jsx_preserve(files: &[PathBuf], tsconfig: Option<&Path>) -> Result<bool, String> {
    let config = match tsconfig {
        Some(config) => Some(config.to_path_buf()),
        None => {
            let files: Vec<PathBuf> = files
                .iter()
                .map(|file| super::paths::canonical(file).unwrap_or_else(|_| file.clone()))
                .collect();
            super::project::find_tsconfig(&files)
        }
    };
    let Some(config) = config else {
        return Ok(false);
    };
    Ok(jsx_option(&config, &mut Vec::new())?
        .is_some_and(|value| value.eq_ignore_ascii_case("preserve")))
}

fn jsx_option(config: &Path, reading: &mut Vec<PathBuf>) -> Result<Option<String>, String> {
    if reading.iter().any(|open| open == config) {
        return Err(format!(
            "{}: circularity detected while resolving configuration",
            config.display()
        ));
    }
    let text = std::fs::read_to_string(config)
        .map_err(|error| format!("{}: {error}", config.display()))?;
    let text = json_text(text.strip_prefix('\u{feff}').unwrap_or(&text));
    let value: serde_json::Value = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", config.display()))?
    };
    if let Some(jsx) = value["compilerOptions"]["jsx"].as_str() {
        return Ok(Some(jsx.to_string()));
    }
    let bases: Vec<&str> = match &value["extends"] {
        serde_json::Value::String(base) => vec![base.as_str()],
        serde_json::Value::Array(bases) => bases.iter().filter_map(|b| b.as_str()).collect(),
        _ => Vec::new(),
    };
    let directory = config.parent().unwrap_or(Path::new("."));
    reading.push(config.to_path_buf());
    let mut found = None;
    for base in bases {
        let path = extended_config(directory, base).ok_or_else(|| {
            format!(
                "{}: cannot find the configuration it extends, {base:?}",
                config.display()
            )
        })?;
        if let Some(jsx) = jsx_option(&path, reading)? {
            found = Some(jsx);
        }
    }
    reading.pop();
    Ok(found)
}

fn extended_config(directory: &Path, base: &str) -> Option<PathBuf> {
    let with_json = |path: &Path| -> Option<PathBuf> {
        if path.is_file() {
            return Some(path.to_path_buf());
        }
        let named = PathBuf::from(format!("{}.json", path.display()));
        (!base.ends_with(".json") && named.is_file()).then_some(named)
    };
    if Path::new(base).is_absolute() || base.starts_with("./") || base.starts_with("../") {
        return with_json(&directory.join(base));
    }
    directory.ancestors().find_map(|ancestor| {
        let module = ancestor.join("node_modules").join(base);
        if let Some(file) = with_json(&module) {
            return Some(file);
        }
        if !module.is_dir() {
            return None;
        }
        let manifest = std::fs::read_to_string(module.join("package.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
        if let Some(field) = manifest
            .as_ref()
            .and_then(|manifest| manifest["tsconfig"].as_str())
            && let Some(file) = with_json(&module.join(field))
        {
            return Some(file);
        }
        Some(module.join("tsconfig.json")).filter(|file| file.is_file())
    })
}

fn json_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut held_comma = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'/') => {
                while chars.next_if(|&next| next != '\n').is_some() {}
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut previous = ' ';
                for next in chars.by_ref() {
                    if previous == '*' && next == '/' {
                        break;
                    }
                    previous = next;
                }
                out.push(' ');
            }
            ',' => {
                if held_comma {
                    out.push(',');
                }
                held_comma = true;
            }
            '}' | ']' => {
                held_comma = false;
                out.push(c);
            }
            c if c.is_whitespace() => out.push(c),
            _ => {
                if held_comma {
                    out.push(',');
                    held_comma = false;
                }
                out.push(c);
                if c == '"' {
                    while let Some(inner) = chars.next() {
                        out.push(inner);
                        match inner {
                            '\\' => out.extend(chars.next()),
                            '"' => break,
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    if held_comma {
        out.push(',');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::json_text;

    #[test]
    fn configuration_text_reads_as_json() {
        let text =
            "{\n  // a comment\n  \"a\": [1, 2,],\n  /* \"b\": 1, */ \"c\": \"x // y, }\",\n}\n";
        let value: serde_json::Value = serde_json::from_str(&json_text(text)).unwrap();
        assert_eq!(value, serde_json::json!({ "a": [1, 2], "c": "x // y, }" }));
    }

    #[test]
    fn an_empty_comment_only_or_bom_prefixed_configuration_is_read() {
        let dir = crate::test_workspace::Workspace::new("config-text");
        let config = dir.join("tsconfig.json");
        for (text, jsx) in [
            ("", None),
            ("// only a comment\n", None),
            (
                "\u{feff}{\"compilerOptions\":{\"jsx\":\"preserve\"}}",
                Some("preserve"),
            ),
        ] {
            std::fs::write(&config, text).unwrap();
            assert_eq!(
                super::jsx_option(&config, &mut Vec::new())
                    .unwrap()
                    .as_deref(),
                jsx,
                "{text:?}"
            );
        }
    }
}
