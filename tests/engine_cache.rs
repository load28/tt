//! The cross-snapshot semantic cache's invalidation contract
//! (`docs/design/compiler-core.md` §11, TASK-126):
//!
//! - an unchanged file's semantics are reused across snapshots;
//! - a dependency's **body-only** change keeps its importers' entries;
//! - a dependency's **exported-declaration** change invalidates exactly
//!   the importers.
//!
//! The cache is observable with or without a TypeScript toolchain: since
//! TASK-124 a missing backend degrades the typed facts instead of failing
//! the pass, so `check()` runs — and counts cache hits — either way.

use std::fs;
use std::path::PathBuf;

use ttc::engine::{CheckRequest, Engine, ProjectOptions};

fn tmpdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tt-cache-{}-{tag}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(unix)]
#[test]
fn source_walk_skips_excluded_names_before_following_links() {
    let dir = tmpdir("excluded-dangling-links");
    let source = dir.join("ok.tt");
    fs::write(&source, "export const ok = 1;\n").unwrap();
    std::os::unix::fs::symlink(dir.join("absent-vendor"), dir.join("node_modules")).unwrap();
    std::os::unix::fs::symlink(dir.join("absent-cache"), dir.join(".cache")).unwrap();

    let mut collected = Vec::new();
    ttc::engine::collect_sources(&dir, false, &mut collected).unwrap();
    assert_eq!(collected, vec![source.clone()]);

    let engine = Engine::new(None);
    let project = engine
        .open_project(
            &[dir.to_string_lossy().to_string()],
            &ProjectOptions::default(),
        )
        .unwrap();
    assert_eq!(project.scan().unwrap(), vec![source]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn body_changes_keep_importers_and_export_changes_invalidate_them() {
    let dir = tmpdir("invalidate");
    let shared = dir.join("shared.tt");
    let user = dir.join("user.tt");
    fs::write(
        &shared,
        "export variant Token { Num(value: number), Eof }\nexport const noise = 1;\n",
    )
    .unwrap();
    fs::write(
        &user,
        "import { Token } from \"./shared.tt\";\n\
         export function f(t: Token): number {\n\
         \x20 return match (t) { Num(value) => value, Eof => 0 };\n\
         }\n",
    )
    .unwrap();

    let engine = Engine::new(None);
    let mut project = engine
        .open_project(
            &[dir.to_string_lossy().to_string()],
            &ProjectOptions::default(),
        )
        .expect("opens without a toolchain (TASK-124)");
    let files = project.initial_files();

    // First pass: everything computes, nothing hits.
    let snapshot = project.update(&files).unwrap();
    project.check(&snapshot, &CheckRequest::default()).unwrap();
    assert_eq!(project.semantic_cache_hits(), 0);

    // Same content: both files hit.
    let snapshot = project.update(&files).unwrap();
    project.check(&snapshot, &CheckRequest::default()).unwrap();
    assert_eq!(project.semantic_cache_hits(), 2);

    // A body-only change to the dependency: shared recomputes (content
    // changed), the importer's key — its own content plus shared's
    // *exports* — is untouched, so user.tt hits.
    fs::write(
        &shared,
        "export variant Token { Num(value: number), Eof }\nexport const noise = 2;\n",
    )
    .unwrap();
    let snapshot = project.update(&files).unwrap();
    project.check(&snapshot, &CheckRequest::default()).unwrap();
    assert_eq!(
        project.semantic_cache_hits(),
        3,
        "the importer survived a dependency body change"
    );

    // An exported-declaration change: the importer's externs differ, so
    // user.tt recomputes too — no stale exhaustiveness model survives.
    fs::write(
        &shared,
        "export variant Token { Num(value: number), Word(text: string), Eof }\nexport const noise = 2;\n",
    )
    .unwrap();
    let snapshot = project.update(&files).unwrap();
    project.check(&snapshot, &CheckRequest::default()).unwrap();
    assert_eq!(
        project.semantic_cache_hits(),
        3,
        "an exported-declaration change invalidated the importer"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_unchanged_projection_is_shared_across_snapshots() {
    let dir = tmpdir("projection");
    let file = dir.join("a.tt");
    fs::write(&file, "export variant E { A(x: number), B }\n").unwrap();

    let engine = Engine::new(None);
    let mut project = engine
        .open_project(
            &[dir.to_string_lossy().to_string()],
            &ProjectOptions::default(),
        )
        .unwrap();
    let files = project.initial_files();
    let first = project.update(&files).unwrap();
    let second = project.update(&files).unwrap();
    // Same content ⇒ the same Arc — the projection was not recomputed.
    assert!(std::sync::Arc::ptr_eq(
        &first.files()[0],
        &second.files()[0]
    ));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_error_node_keeps_its_file_and_other_files_checkable() {
    let dir = tmpdir("partial-snapshot");
    let blocked = dir.join("a-blocked.tt");
    let valid = dir.join("b-valid.tt");
    fs::write(&blocked, "const broken = 1 |> ;\n").unwrap();
    // A rule the tt layer answers on its own. Exhaustiveness would not do:
    // the engine defers that to the checker, so on a machine with no
    // TypeScript toolchain this file would have no diagnostic and the case
    // would fail for a reason that has nothing to do with what it tests
    // (TASK-224). The subject here is the *partial snapshot* — a blocked
    // file keeps its own diagnostics and its neighbour still reports.
    fs::write(
        &valid,
        "variant E { A(value: number), B }\n\
         const value = match (E.A(1)) { A(value) => value, A(value) => 0, B => 1 };\n",
    )
    .unwrap();

    let engine = Engine::new(None);
    let mut project = engine
        .open_project(
            &[dir.to_string_lossy().to_string()],
            &ProjectOptions::default(),
        )
        .unwrap();
    let files = project.initial_files();
    let blocked = blocked.canonicalize().unwrap();
    let valid = valid.canonicalize().unwrap();
    let snapshot = project.update(&files).expect("partial snapshot");
    let checked = project
        .check(
            &snapshot,
            &CheckRequest {
                emit_declarations: false,
                tt_only: true,
            },
        )
        .unwrap();

    assert_eq!(snapshot.files().len(), 2);
    assert_eq!(checked.diagnostics.len(), 2, "{:#?}", checked.diagnostics);
    assert!(checked.diagnostics.iter().any(|d| d.path == blocked));
    assert!(checked.diagnostics.iter().any(|d| d.path == valid));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn snapshot_discovers_transitive_imports_and_reuses_unchanged_documents() {
    let dir = ttc::engine::normalize_document_path(&tmpdir("snapshot-import-graph")).unwrap();
    fs::create_dir_all(dir.join("app")).unwrap();
    fs::create_dir_all(dir.join("domain")).unwrap();
    let entry = dir.join("app/main.tt");
    let model = dir.join("domain/model.tt");
    let leaf = dir.join("domain/leaf.tt");
    fs::write(
        &entry,
        "import {value} from '../domain/model.tt'; export const answer = value;",
    )
    .unwrap();
    fs::write(&model, "export {value} from './leaf.tt';").unwrap();
    fs::write(
        &leaf,
        "import type {answer} from '../app/main.tt'; export const value = 1;",
    )
    .unwrap();
    let mut project = Engine::new(None)
        .open_project(
            &[dir.join("app").to_string_lossy().into_owned()],
            &ProjectOptions::default(),
        )
        .unwrap();
    let roots = project.initial_files();
    assert_eq!(roots.as_slice(), std::slice::from_ref(&entry));
    let first = project.update(&roots).unwrap();
    assert_eq!(
        first.files().len(),
        3,
        "follow a re-export and terminate a cycle"
    );
    let second = project.update(&roots).unwrap();
    for old in first.files() {
        let new = second
            .files()
            .iter()
            .find(|doc| doc.source_path == old.source_path)
            .unwrap();
        assert!(std::sync::Arc::ptr_eq(old, new));
    }
    // A held import edit changes the reachable graph without changing disk.
    project.open_document(entry.clone(), "export const answer = 2;".into());
    let third = project.update(&roots).unwrap();
    assert_eq!(third.files().len(), 1);
    assert_eq!(
        first.files().len(),
        3,
        "previous snapshots remain immutable"
    );
    let unsaved = dir.join("domain/unsaved.tt");
    project.open_document(unsaved.clone(), "export const fresh = 3;".into());
    project.open_document(
        entry.clone(),
        "export {fresh} from '../domain/unsaved.tt';".into(),
    );
    let fourth = project.update(&roots).unwrap();
    assert_eq!(fourth.files().len(), 2);
    assert!(fourth.files().iter().any(|doc| doc.source_path == unsaved));
    assert!(!unsaved.exists());
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn project_scan_follows_directory_symlinks() {
    let dir = ttc::engine::normalize_document_path(&tmpdir("scan-symlink")).unwrap();
    fs::create_dir_all(dir.join("app/nested")).unwrap();
    fs::create_dir_all(dir.join("shared")).unwrap();
    let entry = dir.join("app/main.tt");
    let nested = dir.join("app/nested/model.tt");
    let linked = dir.join("shared/linked.tt");
    for file in [&entry, &nested, &linked] {
        fs::write(file, "export const value = 1;").unwrap();
    }
    std::os::unix::fs::symlink(dir.join("shared"), dir.join("app/shared")).unwrap();
    let project = Engine::new(None)
        .open_project(
            &[entry.to_string_lossy().into_owned()],
            &ProjectOptions::default(),
        )
        .unwrap();
    let mut expected = vec![entry, nested, linked];
    expected.sort();
    assert_eq!(project.scan().unwrap(), expected);
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn source_discovery_visits_directory_identities_once() {
    let dir = ttc::engine::normalize_document_path(&tmpdir("scan-identities")).unwrap();
    fs::create_dir_all(dir.join("app/nested")).unwrap();
    fs::create_dir_all(dir.join("shared")).unwrap();
    let entry = dir.join("app/main.tt");
    let linked = dir.join("shared/linked.tt");
    for file in [&entry, &linked] {
        fs::write(file, "export const value = 1;").unwrap();
    }
    // A self edge, an ancestor edge, and two aliases of an external source
    // directory must all use the same directory identity admission rule.
    std::os::unix::fs::symlink(dir.join("app"), dir.join("app/self")).unwrap();
    std::os::unix::fs::symlink(dir.join("app"), dir.join("app/nested/back")).unwrap();
    std::os::unix::fs::symlink(dir.join("shared"), dir.join("app/shared-a")).unwrap();
    std::os::unix::fs::symlink(dir.join("shared"), dir.join("app/shared-b")).unwrap();
    let mut collected = Vec::new();
    ttc::engine::collect_sources(&dir.join("app"), false, &mut collected).unwrap();
    let mut identities: Vec<_> = collected
        .iter()
        .map(|file| fs::canonicalize(file).unwrap())
        .collect();
    identities.sort();
    let mut expected = vec![entry.clone(), linked];
    expected.sort();
    assert_eq!(identities, expected);
    // CLI output ownership still uses logical input paths.
    assert!(
        collected
            .iter()
            .all(|file| file.starts_with(dir.join("app")))
    );
    let project = Engine::new(None)
        .open_project(
            &[entry.to_string_lossy().into_owned()],
            &ProjectOptions::default(),
        )
        .unwrap();
    assert_eq!(project.scan().unwrap(), expected);
    assert_eq!(project.scan().unwrap(), expected, "visits are per scan");
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn project_scan_excludes_output_directory_aliases_and_descendants() {
    let dir = ttc::engine::normalize_document_path(&tmpdir("scan-output-alias")).unwrap();
    fs::create_dir_all(dir.join("app/build/nested")).unwrap();
    let entry = dir.join("app/main.tt");
    fs::write(&entry, "export const value = 1;").unwrap();
    fs::write(dir.join("app/build/generated.tt"), "export const x = 1;").unwrap();
    fs::write(dir.join("app/build/nested/deep.tt"), "export const x = 2;").unwrap();
    std::os::unix::fs::symlink(dir.join("app/build"), dir.join("app/alias")).unwrap();
    std::os::unix::fs::symlink(dir.join("app/build/nested"), dir.join("app/deep")).unwrap();
    let project = Engine::new(None)
        .open_project(
            &[entry.to_string_lossy().into_owned()],
            &ProjectOptions {
                out_dir: Some(dir.join("app/build")),
                ..ProjectOptions::default()
            },
        )
        .unwrap();
    assert_eq!(project.initial_files(), vec![entry.clone()]);
    let scanned = project.scan().unwrap();
    assert_eq!(scanned, vec![entry.clone()]);
    let watched = project.watch_paths().unwrap();
    assert!(watched.contains(&entry));
    for alias in ["app/alias/generated.tt", "app/deep/deep.tt"] {
        let logical = dir.join(alias);
        let identity = fs::canonicalize(&logical).unwrap();
        for paths in [&scanned, &watched] {
            assert!(!paths.contains(&logical), "output alias leaked: {alias}");
            assert!(
                !paths.contains(&identity),
                "output identity leaked: {alias}"
            );
        }
    }
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn project_scan_deduplicates_file_symlinks() {
    let dir = ttc::engine::normalize_document_path(&tmpdir("scan-file-alias")).unwrap();
    let entry = dir.join("main.tt");
    fs::write(&entry, "export const value = 1;").unwrap();
    std::os::unix::fs::symlink(&entry, dir.join("alias.tt")).unwrap();
    let project = Engine::new(None)
        .open_project(
            &[entry.to_string_lossy().into_owned()],
            &ProjectOptions::default(),
        )
        .unwrap();
    assert_eq!(project.scan().unwrap(), vec![entry]);
    fs::remove_dir_all(dir).unwrap();
}
