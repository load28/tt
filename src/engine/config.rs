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
    if tsconfig.is_none() && solution_style(&config)? {
        return Err(format!(
            "{}: a solution-style configuration (`\"files\": []` with `references`) leaves \
             each file to the referenced project that contains it",
            config.display()
        ));
    }
    Ok(jsx_option(&config, &mut Vec::new())?
        .flatten()
        .is_some_and(|value| value.eq_ignore_ascii_case("preserve")))
}

fn solution_style(config: &Path) -> Result<bool, String> {
    let value = read_config(config)?;
    Ok(value["files"].as_array().is_some_and(Vec::is_empty)
        && value["references"]
            .as_array()
            .is_some_and(|references| !references.is_empty()))
}

fn read_config(config: &Path) -> Result<serde_json::Value, String> {
    let text = std::fs::read_to_string(config)
        .map_err(|error| format!("{}: {error}", config.display()))?;
    let text = json_text(text.strip_prefix('\u{feff}').unwrap_or(&text));
    if text.trim().is_empty() {
        Ok(serde_json::json!({}))
    } else {
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", config.display()))
    }
}

const JSX_VALUES: [&str; 5] = [
    "preserve",
    "react-native",
    "react-jsx",
    "react-jsxdev",
    "react",
];

fn jsx_option(config: &Path, reading: &mut Vec<PathBuf>) -> Result<Option<Option<String>>, String> {
    if reading.iter().any(|open| open == config) {
        return Err(format!(
            "{}: circularity detected while resolving configuration",
            config.display()
        ));
    }
    let value = read_config(config)?;
    match value["compilerOptions"].get("jsx") {
        Some(serde_json::Value::Null) => return Ok(Some(None)),
        Some(serde_json::Value::String(jsx)) => {
            if !JSX_VALUES
                .iter()
                .any(|known| known.eq_ignore_ascii_case(jsx))
            {
                return Err(format!(
                    "{}: Argument for '--jsx' option must be: {}.",
                    config.display(),
                    JSX_VALUES.map(|known| format!("'{known}'")).join(", ")
                ));
            }
            return Ok(Some(Some(jsx.clone())));
        }
        Some(_) => {
            return Err(format!(
                "{}: Compiler option 'jsx' requires a value of type enum.",
                config.display()
            ));
        }
        None => {}
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
        if let Some(setting) = jsx_option(&path, reading)? {
            found = Some(setting);
        }
    }
    reading.pop();
    Ok(found)
}

fn extended_config(directory: &Path, base: &str) -> Option<PathBuf> {
    let base = base.replace('\\', "/");
    let base = base.as_str();
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
    let (name, subpath) = package_name_and_subpath(base)?;
    directory.ancestors().find_map(|ancestor| {
        let package = ancestor.join("node_modules").join(name);
        let manifest = std::fs::read_to_string(package.join("package.json")).ok();
        let fields = manifest.as_deref().and_then(|text| {
            serde_json::from_str::<
                std::collections::HashMap<String, Box<serde_json::value::RawValue>>,
            >(text)
            .ok()
        });
        if let Some(exports) = fields.as_ref().and_then(|fields| fields.get("exports")) {
            return exports_target(exports.get(), &subpath)
                .map(|target| package.join(target))
                .filter(|file| file.is_file());
        }
        let module = ancestor.join("node_modules").join(base);
        if let Some(file) = with_json(&module) {
            return Some(file);
        }
        if !module.is_dir() {
            return None;
        }
        if let Some(field) = fields
            .as_ref()
            .filter(|_| subpath == ".")
            .and_then(|fields| fields.get("tsconfig"))
            .and_then(|field| serde_json::from_str::<String>(field.get()).ok())
            && let Some(file) = with_json(&module.join(field))
        {
            return Some(file);
        }
        Some(module.join("tsconfig.json")).filter(|file| file.is_file())
    })
}

fn package_name_and_subpath(specifier: &str) -> Option<(&str, String)> {
    let mut parts = specifier.splitn(if specifier.starts_with('@') { 3 } else { 2 }, '/');
    let first = parts.next()?;
    let name_len = if specifier.starts_with('@') {
        first.len() + 1 + parts.next()?.len()
    } else {
        first.len()
    };
    let rest = parts.next();
    Some((
        &specifier[..name_len],
        rest.map_or_else(|| ".".to_owned(), |rest| format!("./{rest}")),
    ))
}

const CONFIG_CONDITIONS: [&str; 4] = ["default", "require", "types", "node"];

fn exports_target(exports: &str, subpath: &str) -> Option<String> {
    let exports = exports.trim_start();
    if !exports.starts_with('{') {
        return (subpath == ".")
            .then(|| export_target(exports, None))
            .flatten();
    }
    let entries = ordered_entries(exports)?;
    if !entries.iter().all(|(key, _)| key.starts_with('.')) {
        return (subpath == ".")
            .then(|| export_target(exports, None))
            .flatten();
    }
    if let Some((_, target)) = entries.iter().find(|(key, _)| key == subpath) {
        return export_target(target, None);
    }
    entries
        .iter()
        .filter_map(|(key, target)| {
            let (prefix, suffix) = key.split_once('*')?;
            (subpath.len() >= prefix.len() + suffix.len()
                && subpath.starts_with(prefix)
                && subpath.ends_with(suffix))
            .then(|| {
                (
                    prefix.len(),
                    &subpath[prefix.len()..subpath.len() - suffix.len()],
                    target,
                )
            })
        })
        .max_by_key(|(prefix, ..)| *prefix)
        .and_then(|(_, star, target)| export_target(target, Some(star)))
}

fn export_target(target: &str, star: Option<&str>) -> Option<String> {
    let target = target.trim_start();
    match target.as_bytes().first()? {
        b'"' => {
            let path: String = serde_json::from_str(target).ok()?;
            path.starts_with("./")
                .then(|| star.map_or_else(|| path.clone(), |star| path.replace('*', star)))
        }
        b'[' => serde_json::from_str::<Vec<Box<serde_json::value::RawValue>>>(target)
            .ok()?
            .iter()
            .find_map(|element| export_target(element.get(), star)),
        b'{' => ordered_entries(target)?
            .into_iter()
            .filter(|(condition, _)| CONFIG_CONDITIONS.contains(&condition.as_str()))
            .find_map(|(_, value)| export_target(value, star)),
        _ => None,
    }
}

fn ordered_entries(object: &str) -> Option<Vec<(String, &str)>> {
    let bytes = object.as_bytes();
    let mut at = object.find('{')? + 1;
    let skip_space = |at: &mut usize| {
        while bytes.get(*at).is_some_and(u8::is_ascii_whitespace) {
            *at += 1;
        }
    };
    let string_end = |start: usize| -> Option<usize> {
        let mut index = start + 1;
        while index < bytes.len() {
            match bytes[index] {
                b'\\' => index += 2,
                b'"' => return Some(index + 1),
                _ => index += 1,
            }
        }
        None
    };
    let mut entries = Vec::new();
    loop {
        skip_space(&mut at);
        match bytes.get(at)? {
            b'}' => return Some(entries),
            b',' => {
                at += 1;
                continue;
            }
            b'"' => {}
            _ => return None,
        }
        let key_end = string_end(at)?;
        let key: String = serde_json::from_str(&object[at..key_end]).ok()?;
        at = key_end;
        skip_space(&mut at);
        if bytes.get(at) != Some(&b':') {
            return None;
        }
        at += 1;
        skip_space(&mut at);
        let start = at;
        let mut depth = 0usize;
        while at < bytes.len() {
            match bytes[at] {
                b'"' => {
                    at = string_end(at)?;
                    continue;
                }
                b'{' | b'[' => depth += 1,
                b'}' | b']' if depth == 0 => break,
                b'}' | b']' => depth -= 1,
                b',' if depth == 0 => break,
                _ => {}
            }
            at += 1;
        }
        entries.push((key, object[start..at].trim_end()));
    }
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
    fn a_configuration_extends_what_typescript_resolves_through_package_exports() {
        let dir = crate::test_workspace::Workspace::new("config-extends-exports");
        let write = |path: &str, text: &str| {
            let path = dir.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        let preserve = "{\"compilerOptions\":{\"jsx\":\"preserve\"}}";
        write(
            "node_modules/conditional/package.json",
            "{\"exports\":{\"./strict\":{\"import\":\"./missing.json\",\"require\":\"./cfg/strict.json\"}}}",
        );
        write("node_modules/conditional/cfg/strict.json", preserve);
        write(
            "node_modules/root/package.json",
            "{\"exports\":{\".\":\"./c.json\"}}",
        );
        write("node_modules/root/c.json", preserve);
        write(
            "node_modules/@scope/pattern/package.json",
            "{\"exports\":{\"./*\":\"./configs/*.json\"}}",
        );
        write("node_modules/@scope/pattern/configs/x.json", preserve);
        write("base.json", preserve);
        let config = dir.join("tsconfig.json");
        for base in [
            "conditional/strict",
            "root",
            "@scope/pattern/x",
            ".\\\\base.json",
        ] {
            std::fs::write(&config, format!("{{\"extends\":\"{base}\"}}")).unwrap();
            assert_eq!(
                super::jsx_option(&config, &mut Vec::new())
                    .unwrap()
                    .flatten()
                    .as_deref(),
                Some("preserve"),
                "{base}"
            );
        }
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
                    .flatten()
                    .as_deref(),
                jsx,
                "{text:?}"
            );
        }
    }

    #[test]
    fn a_null_jsx_unsets_the_value_a_configuration_extends() {
        let dir = crate::test_workspace::Workspace::new("config-jsx-null");
        let write = |name: &str, text: &str| std::fs::write(dir.join(name), text).unwrap();
        write("base.json", "{\"compilerOptions\":{\"jsx\":\"preserve\"}}");
        write("unset.json", "{\"compilerOptions\":{\"jsx\":null}}");
        let config = dir.join("tsconfig.json");
        for (text, preserve) in [
            (
                "{\"extends\":\"./base.json\",\"compilerOptions\":{\"jsx\":null}}",
                false,
            ),
            ("{\"extends\":[\"./base.json\",\"./unset.json\"]}", false),
            ("{\"extends\":[\"./unset.json\",\"./base.json\"]}", true),
        ] {
            write("tsconfig.json", text);
            assert_eq!(
                super::jsx_preserve(&[], Some(&config)),
                Ok(preserve),
                "{text}"
            );
        }
    }

    #[test]
    fn a_jsx_value_typescript_rejects_is_an_error() {
        let dir = crate::test_workspace::Workspace::new("config-jsx-invalid");
        let config = dir.join("tsconfig.json");
        for (text, message) in [
            (
                "{\"compilerOptions\":{\"jsx\":\"bogus\"}}",
                "Argument for '--jsx' option must be: 'preserve', 'react-native', 'react-jsx', 'react-jsxdev', 'react'.",
            ),
            (
                "{\"compilerOptions\":{\"jsx\":5}}",
                "Compiler option 'jsx' requires a value of type enum.",
            ),
        ] {
            std::fs::write(&config, text).unwrap();
            let error = super::jsx_preserve(&[], Some(&config)).unwrap_err();
            assert!(error.ends_with(message), "{error}");
        }
    }

    #[test]
    fn a_solution_style_configuration_names_no_jsx_option_for_a_file() {
        let dir = crate::test_workspace::Workspace::new("config-solution");
        let write = |name: &str, text: &str| std::fs::write(dir.join(name), text).unwrap();
        write(
            "tsconfig.app.json",
            "{\"compilerOptions\":{\"jsx\":\"preserve\"},\"include\":[\"src\"]}",
        );
        write(
            "tsconfig.json",
            "{\"files\":[],\"references\":[{\"path\":\"./tsconfig.app.json\"}]}",
        );
        std::fs::create_dir_all(dir.join("src")).unwrap();
        write("src/a.tt", "");
        let file = dir.join("src/a.tt");
        let error = super::jsx_preserve(std::slice::from_ref(&file), None).unwrap_err();
        assert!(error.contains("solution-style"), "{error}");
        assert_eq!(
            super::jsx_preserve(
                std::slice::from_ref(&file),
                Some(&dir.join("tsconfig.app.json"))
            ),
            Ok(true)
        );
    }
}
