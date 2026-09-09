//! Symbol, emit-map, sidecar, and JSON presentation modes.

use super::*;

/// Prints each input file's declarations and direct tt imports as JSON.
pub(super) fn symbols_mode(jobs: &[Job]) -> ExitCode {
    let mut entries: Vec<String> = Vec::new();
    let mut failed = false;
    for job in jobs {
        let filename = job.file.display().to_string();
        let source = match fs::read_to_string(&job.file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("ttc: {filename}: {e}");
                failed = true;
                continue;
            }
        };
        let mut entry = format!("{{\"file\":{}", json_str(&filename));
        entry.push_str(",\"variants\":");
        entry.push_str(&variants_json(&source, &ttc::variant_symbols(&source)));
        entry.push_str(",\"imports\":[");
        let dir = job.file.parent().unwrap_or(Path::new("."));
        let imports = ttc::tt_imports(&source)
            .iter()
            .map(|import| {
                let mut o = format!("{{\"specifier\":{}", json_str(&import.specifier));
                o.push_str(",\"names\":");
                o.push_str(&names_json(&import.names));
                let target = dir.join(&import.specifier);
                match fs::read_to_string(&target) {
                    Ok(imported_src) => {
                        o.push_str(&format!(
                            ",\"resolved\":{}",
                            json_str(&target.display().to_string())
                        ));
                        let exported: Vec<VariantSymbol> = ttc::variant_symbols(&imported_src)
                            .into_iter()
                            .filter(|e| e.exported)
                            .collect();
                        o.push_str(",\"variants\":");
                        o.push_str(&variants_json(&imported_src, &exported));
                    }
                    Err(_) => o.push_str(",\"resolved\":null,\"variants\":[]"),
                }
                o.push('}');
                o
            })
            .collect::<Vec<_>>();
        entry.push_str(&imports.join(","));
        entry.push_str("]}");
        entries.push(entry);
    }
    crate::out::line(&format!("[{}]", entries.join(",")));
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// `--emit-map`: prints, as a JSON array on stdout, each input file's
/// emitted TypeScript and the source<->output byte mappings of every chunk
/// copied verbatim from the source (`ttc::emit_mapped`). Parse + emit only —
/// no tt-level checks, no verification, `.tt`/`@tt/std` specifiers left
/// untouched — so a buffer mid-edit still emits. This is the feed for the
/// language server's virtual TypeScript documents (TASK-050).
pub(super) fn emit_map_mode(jobs: &[Job]) -> ExitCode {
    let mut entries: Vec<String> = Vec::new();
    let mut failed = false;
    for job in jobs {
        let filename = job.file.display().to_string();
        let source = match fs::read_to_string(&job.file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("ttc: {filename}: {e}");
                failed = true;
                continue;
            }
        };
        let mapped = ttc::emit_mapped(&source);
        let mappings = mapped
            .mappings
            .iter()
            .map(|m| format!("{{\"src\":{},\"out\":{},\"len\":{}}}", m.src, m.out, m.len))
            .collect::<Vec<_>>()
            .join(",");
        entries.push(format!(
            "{{\"file\":{},\"code\":{},\"mappings\":[{}]}}",
            json_str(&filename),
            json_str(&mapped.code),
            mappings
        ));
    }
    crate::out::line(&format!("[{}]", entries.join(",")));
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// `--sidecar <dir>`: writes `<name>.tt.d.ts` and `<name>.tt.d.ts.map` next
/// to each input `.tt`, from the declarations tsc emitted for that module
/// (`<dir>/<name>.d.ts`, produced with `--emitDeclarationOnly` over ttc's
/// output). The map's `sources` is the `.tt` file, so an editor's "go to
/// definition" from a `.ts` importer lands in the original — not in the
/// generated declarations. Compiles nothing.
pub(super) fn sidecar_mode(jobs: &[Job], decl_dir: &Path) -> ExitCode {
    let mut failed = false;
    for job in jobs {
        let Some(stem) = job
            .file
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
        else {
            continue;
        };
        let Some(file_name) = job
            .file
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
        else {
            continue;
        };

        let source = match fs::read_to_string(&job.file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("ttc: {}: {e}", job.file.display());
                failed = true;
                continue;
            }
        };
        let decl_path = decl_dir.join(format!("{stem}.d.ts"));
        let declarations = match fs::read_to_string(&decl_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("ttc: {}: {e}", decl_path.display());
                failed = true;
                continue;
            }
        };

        // `-o` puts the declarations in their own tree (mirroring the input
        // layout); without it they sit next to the source.
        let dts_path = job.out_path.with_file_name(format!("{file_name}.d.ts"));
        let map_path = job.out_path.with_file_name(format!("{file_name}.d.ts.map"));
        let dir = dts_path.parent().unwrap_or(Path::new(".")).to_path_buf();
        if let Err(e) = fs::create_dir_all(&dir) {
            eprintln!("ttc: {}: {e}", dir.display());
            failed = true;
            continue;
        }

        // The map's `sources` is read relative to the map itself, so it has
        // to point back across whatever distance `-o` introduced.
        let sidecar = ttc::build_sidecar(&source, &declarations, &relative_path(&dir, &job.file));
        if let Err(e) = fs::write(&dts_path, &sidecar.declarations) {
            eprintln!("ttc: {}: {e}", dts_path.display());
            failed = true;
            continue;
        }
        if let Err(e) = fs::write(&map_path, &sidecar.map) {
            eprintln!("ttc: {}: {e}", map_path.display());
            failed = true;
            continue;
        }
        eprintln!("ttc: {} → {}", job.file.display(), dts_path.display());
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Path from `from_dir` to `to_file`, `/`-separated — the form a source map
/// needs for its `sources`.
pub(super) fn relative_path(from_dir: &Path, to_file: &Path) -> String {
    // Canonicalize both or neither: an output directory may not exist yet,
    // and mixing an absolute path with a relative one yields nonsense.
    let (from, to) = match (from_dir.canonicalize(), to_file.canonicalize()) {
        (Ok(from), Ok(to)) => (from, to),
        _ => (from_dir.to_path_buf(), to_file.to_path_buf()),
    };

    let from_parts: Vec<_> = from.components().collect();
    let to_parts: Vec<_> = to.components().collect();
    let shared = from_parts
        .iter()
        .zip(&to_parts)
        .take_while(|(a, b)| a == b)
        .count();

    let mut parts: Vec<String> = vec!["..".to_string(); from_parts.len() - shared];
    parts.extend(
        to_parts[shared..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().to_string()),
    );
    if parts.is_empty() {
        return ".".to_string();
    }
    parts.join("/")
}

pub(super) fn variants_json(source: &str, symbols: &[VariantSymbol]) -> String {
    let objects = symbols
        .iter()
        .map(|e| {
            let (line, col) = ttc::line_col(source, e.offset);
            let cases = e
                .cases
                .iter()
                .map(|c| {
                    let (line, col) = ttc::line_col(source, c.offset);
                    let fields = match &c.fields {
                        None => "null".to_string(),
                        Some(fields) => format!(
                            "[{}]",
                            fields
                                .iter()
                                .map(|f| format!(
                                    "{{\"name\":{},\"optional\":{},\"type\":{}}}",
                                    json_str(&f.name),
                                    f.optional,
                                    json_str(&f.ty)
                                ))
                                .collect::<Vec<_>>()
                                .join(",")
                        ),
                    };
                    format!(
                        "{{\"tag\":{},\"line\":{line},\"col\":{col},\"fields\":{fields}}}",
                        json_str(&c.tag)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{{\"name\":{},\"exported\":{},\"generics\":{},\"line\":{line},\"col\":{col},\"cases\":[{cases}]}}",
                json_str(&e.name),
                e.exported,
                json_str(&e.generics)
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", objects.join(","))
}

pub(super) fn names_json(names: &TtImportNames) -> String {
    match names {
        TtImportNames::Namespace(ns) => {
            format!("{{\"kind\":\"namespace\",\"name\":{}}}", json_str(ns))
        }
        TtImportNames::Named(entries) => format!(
            "{{\"kind\":\"named\",\"entries\":[{}]}}",
            entries
                .iter()
                .map(|(name, alias)| format!(
                    "{{\"name\":{},\"alias\":{}}}",
                    json_str(name),
                    alias.as_deref().map_or("null".to_string(), json_str)
                ))
                .collect::<Vec<_>>()
                .join(",")
        ),
        TtImportNames::None => "{\"kind\":\"none\"}".to_string(),
    }
}

/// Minimal JSON string encoding (quotes, backslashes, control characters).
pub(super) fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
