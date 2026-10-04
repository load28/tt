//! Recovery selection, byte-preserving source masking, and variant declarations.

use crate::{AnchorKind, Diagnostic, DiagnosticCode, EmitAnchor, MappedEmit, ast, parser};

fn overwrite_recovery(source: &mut [u8], start: usize, end: usize, replacement: &str) {
    let start = start.min(source.len());
    let end = end.min(source.len()).max(start);
    source[start..end].fill(b' ');
    let bytes = replacement.as_bytes();
    let count = bytes.len().min(end - start);
    source[start..start + count].copy_from_slice(&bytes[..count]);
}

/// The constructs a reported diagnostic lets the typed projection
/// substitute: a tt value whose placement the plan rejects, a discarded
/// `result` block, and an invalid variant field type. Each is an
/// expression (or a type) whose replacement keeps the file's other code
/// checkable.
pub(super) fn recoverable_constructs(diagnostics: &[Diagnostic]) -> Vec<ast::RecoveryNode> {
    diagnostics
        .iter()
        .filter_map(|diagnostic| {
            let span = ast::Span {
                start: diagnostic.start?,
                end: diagnostic.end?,
            };
            let kind = match diagnostic.code {
                DiagnosticCode::TryPlacement
                | DiagnosticCode::TryCrossesValueRegion
                | DiagnosticCode::MatchPlacement
                | DiagnosticCode::ResultValueDiscarded => ast::RecoveryKind::Expression,
                DiagnosticCode::VariantInvalidFieldType => ast::RecoveryKind::Type,
                _ => return None,
            };
            Some(ast::RecoveryNode { span, kind })
        })
        .collect()
}

/// Keeps the outer error node when parser recovery found nested symptoms
/// inside it. This is the same synchronization rule as an error AST node:
/// one placeholder owns one malformed construct. `nodes` is sorted by start,
/// outermost first.
pub(super) fn outermost_recoveries(nodes: Vec<ast::RecoveryNode>) -> Vec<ast::RecoveryNode> {
    let mut selected: Vec<ast::RecoveryNode> = Vec::new();
    for node in nodes {
        if selected
            .last()
            .is_some_and(|outer| node.span.end <= outer.span.end)
        {
            continue;
        }
        selected.push(node);
    }
    selected
}

pub(super) fn recover_source(source: &str, selected: &[ast::RecoveryNode]) -> String {
    let mut recovered = source.as_bytes().to_vec();
    for node in selected {
        let replacement = match &node.kind {
            ast::RecoveryKind::Expression => {
                // The placeholder is `any` wherever it fits, so the code that
                // uses the recovered value has no checker consequence of it.
                let width = node.span.end.saturating_sub(node.span.start);
                let replacement = if width >= "undefined as any".len() {
                    "undefined as any"
                } else if width >= "0 as any".len() {
                    "0 as any"
                } else {
                    "0"
                };
                overwrite_recovery(&mut recovered, node.span.start, node.span.end, replacement);
                continue;
            }
            ast::RecoveryKind::ListElement => "",
            ast::RecoveryKind::MatchArms(_) => {
                unreachable!("match recovery is flattened by the parser")
            }
            ast::RecoveryKind::Statement | ast::RecoveryKind::VariantDecl { .. } => ";",
            ast::RecoveryKind::Type => "any",
            ast::RecoveryKind::OperandHead => "void",
        };
        overwrite_recovery(&mut recovered, node.span.start, node.span.end, replacement);
    }
    // Every overwrite replaces a whole node's byte range with ASCII, and
    // a node's range is a char boundary on both ends, so what is left is
    // still the UTF-8 it started as.
    String::from_utf8(recovered).expect("recovery replaces whole nodes with ASCII")
}

/// Declares each recovered variant's name with the error type, `any`, as
/// both the type and the value a variant declares: its cases could not be
/// read, so nothing that uses it has a checker consequence of it, as for a
/// recovered expression. The declarations are glue after the module, owned
/// by the variant's source range, so the placeholder's width does not bound
/// them.
pub(super) fn declare_recovered_variants(
    mut emit: MappedEmit,
    selected: &[ast::RecoveryNode],
) -> MappedEmit {
    for node in selected {
        let ast::RecoveryKind::VariantDecl {
            name,
            generics,
            exported,
        } = &node.kind
        else {
            continue;
        };
        if parser::is_reserved(name) {
            continue;
        }
        let export = if *exported { "export " } else { "" };
        if !emit.code.is_empty() && !emit.code.ends_with('\n') {
            emit.code.push('\n');
        }
        let out = emit.code.len();
        emit.code.push_str(&format!(
            "{export}declare const {name}: any;\n{export}type {name}{generics} = any;\n"
        ));
        emit.anchors.push(EmitAnchor {
            out,
            end: emit.code.len(),
            src: node.span.start,
            src_end: node.span.end,
            owner_end: node.span.end,
            context: None,
            kind: AnchorKind::Variant,
        });
    }
    emit
}
