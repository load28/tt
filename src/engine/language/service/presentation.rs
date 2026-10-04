//! Pure hover, documentation, and signature-label presentation.

pub(in super::super) fn split_hover(contents: &serde_json::Value) -> (String, String) {
    let value = |contents: &serde_json::Value| {
        contents["value"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    match contents {
        serde_json::Value::String(markdown) => split_markdown_hover(markdown),
        serde_json::Value::Object(_) if contents["kind"] == "markdown" => {
            split_markdown_hover(contents["value"].as_str().unwrap_or_default())
        }
        serde_json::Value::Object(_) => (value(contents), String::new()),
        _ => (String::new(), String::new()),
    }
}

fn split_markdown_hover(markdown: &str) -> (String, String) {
    let trimmed = markdown.trim();
    if let Some(rest) = trimmed.strip_prefix("```")
        && let Some(newline) = rest.find('\n')
    {
        let body = &rest[newline + 1..];
        let (code, prose) = match body.find("\n```") {
            Some(close) => {
                let after = &body[close + 4..];
                (
                    &body[..close],
                    after.find('\n').map_or("", |line| &after[line + 1..]),
                )
            }
            None => (body.strip_suffix("```").unwrap_or(body), ""),
        };
        return (code.trim().to_string(), prose.trim().to_string());
    }
    (String::new(), trimmed.to_string())
}

/// Documentation as plain text, whichever shape the server used.
pub(in super::super) fn docs_text(documentation: &serde_json::Value) -> String {
    match documentation {
        serde_json::Value::String(s) => s.trim().to_string(),
        value => value["value"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

/// Where a parameter's label sits inside its signature — the span form the
/// presentation needs, computed from the substring form when the server
/// used that.
pub(in super::super) fn parameter_span(signature: &str, label: &serde_json::Value) -> (u32, u32) {
    if let Some(span) = label.as_array()
        && span.len() == 2
    {
        return (
            span[0].as_u64().unwrap_or(0) as u32,
            span[1].as_u64().unwrap_or(0) as u32,
        );
    }
    let Some(text) = label.as_str() else {
        return (0, 0);
    };
    // The span is in UTF-16 units of the label string.
    match signature.find(text) {
        Some(byte) => {
            let start = signature[..byte].encode_utf16().count();
            (start as u32, (start + text.encode_utf16().count()) as u32)
        }
        None => (0, 0),
    }
}
