//! Match declaration and external-variant projection.

use super::*;

/// Matches the compiler's emitted declarations back to the snapshot's files.
/// Only requested files are kept; the rest only support graph resolution.
pub(crate) fn match_declarations(
    snapshot: &Snapshot,
    answers: &Answers,
    requested: &HashSet<PathBuf>,
) -> Declarations {
    // The standard library's own declarations, so a consumer running plain
    // tsc can map every `@tt/std` entry to them. They are the package's
    // declaration files, TypeScript's emit of its sources: the package is
    // served under `node_modules`, where a configured program holds it as an
    // external library and emits nothing for it.
    let std = projection::served_std_packages(snapshot.files())
        .into_iter()
        .flat_map(|package| package.modules())
        .map(|module| StdDeclaration {
            module: *module,
            text: module.declaration().to_string(),
        })
        .collect();
    let placeholders = reaches_placeholders(snapshot);
    let modules = answers
        .declarations
        .iter()
        .filter_map(|declaration| {
            let file = snapshot
                .files()
                .iter()
                .find(|f| projection::declaration_path_of(f) == declaration.path)
                .filter(|f| {
                    requested.contains(&f.source_path) && !placeholders.contains(&f.source_path)
                })?;
            Some(ModuleDeclaration {
                file: file.clone(),
                text: declaration.text.clone(),
            })
        })
        .collect();
    Declarations { std, modules }
}

/// The files whose declarations the compiler emitted against a placeholder:
/// a projection whose declarations are not the file's own (an unparsed one,
/// one where a recovery stands for a declaration, or a blocked file served
/// as an empty module), and every file whose `.tt` imports reach one. Their
/// declarations are not written, so the previous ones stand until the
/// source is fixed. A recovered expression leaves the declarations around
/// it the file's own.
fn reaches_placeholders(snapshot: &Snapshot) -> HashSet<PathBuf> {
    let mut reached: HashSet<PathBuf> = snapshot
        .files()
        .iter()
        .filter(|file| file.unparsed || file.recovered_declaration)
        .map(|file| file.source_path.clone())
        .chain(
            snapshot
                .blocked()
                .iter()
                .map(|file| file.source_path.clone()),
        )
        .collect();
    loop {
        let before = reached.len();
        for file in snapshot.files() {
            if reached.contains(&file.source_path) {
                continue;
            }
            let directory = file
                .source_path
                .parent()
                .unwrap_or(std::path::Path::new("."));
            if file.tt_imports().iter().any(|import| {
                directory
                    .join(&import.specifier)
                    .canonicalize()
                    .is_ok_and(|target| reached.contains(&target))
            }) {
                reached.insert(file.source_path.clone());
            }
        }
        if reached.len() == before {
            return reached;
        }
    }
}

/// The variant declarations one file's direct `.tt` imports bring into scope,
/// preferring the snapshot's own text for a file it holds — the same
/// 1-hop collection every other surface does, so an imported variant is known
/// under the name the import gave it.
/// The imported declarations in `file`'s scope, read from the snapshot's
/// cached per-file symbols where the import target is in the snapshot —
/// a target that did not change is never re-parsed — and from disk
/// otherwise.
pub(crate) fn externs_of(
    snapshot: &Snapshot,
    file: &ProjectedDocument,
) -> Vec<crate::resolve::ImportedVariant> {
    super::super::language::externs_from(&file.source_path, file.tt_imports(), &|target| {
        snapshot
            .files()
            .iter()
            .find(|f| f.source_path == target)
            .map(|f| f.exported_variant_symbols().to_vec())
            .or_else(|| {
                snapshot
                    .blocked()
                    .iter()
                    .find(|f| f.source_path == target)
                    .map(|f| f.exported_variant_symbols().to_vec())
            })
            .or_else(|| {
                let text = std::fs::read_to_string(target).ok()?;
                Some(crate::exported_variant_symbols_with_kind(
                    &text,
                    crate::SourceKind::from_path(target).unwrap_or_default(),
                ))
            })
    })
}

/// A covered literal as it reads in a message.
pub(super) fn display_literal(literal: &crate::Literal) -> String {
    match literal {
        crate::Literal::String(s) => format!("{s:?}"),
        crate::Literal::Number(n) => crate::ast::js_number_string(*n),
        crate::Literal::BigInt(d) => format!("{d}n"),
        crate::Literal::Boolean(b) => b.to_string(),
    }
}
