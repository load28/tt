use std::collections::BTreeSet;

use super::*;

#[test]
fn the_surface_fixture_covers_every_ast_segment_kind() {
    fn segment_name(segment: &Segment) -> &'static str {
        match segment {
            Segment::Verbatim(_) => "verbatim",
            Segment::Variant(_) => "variant",
            Segment::Match(_) => "match",
            Segment::TupleMatch(_) => "tuple-match",
            Segment::Try(_) => "try-statement",
            Segment::TryExpr(_) => "try-expression",
            Segment::LetElse(_) => "let-else",
            Segment::IfLet(_) => "if-let",
            Segment::TtImport(_) => "tt-import",
            Segment::ValModifier(_) => "val",
            Segment::Template(_) => "template",
            Segment::Pipe(pipe) if pipe.head.is_some() => "pipeline",
            Segment::Pipe(_) => "flow",
            Segment::ResultBlock(_) => "result",
        }
    }

    let source = r#"import type { Other } from "./other.tt";
variant E { A(value: number), B }
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
declare const node: E;
declare function load(): R<number>;
val const stable = 1;
function probe() {
  const direct = try load();
  const nested = [try load()];
  const A(value) = node else { return 0; };
  if let A(value: again) = node { use(again); } else { use(value); }
  const single = match (node) { A(value) => value, B => 0 };
  const tuple = match (node, node) { (A(value), _) => value, (_, _) => 0 };
  const piped = 1 |> ((value: number) => value + 1);
  const composed = flow |> ((value: number) => value + 1);
  const computed = result { const value = try load(); return value; };
  return `${match (node) { A => "a", B => "b" }}:${direct}:${nested}:${stable}:${piped}:${composed}:${computed}`;
}
"#;
    let program = parse(source);
    assert!(program.malformed.is_empty(), "{:#?}", program.malformed);
    let mut found = BTreeSet::new();
    visit_programs(&program, &mut |program| {
        found.extend(program.segments.iter().map(segment_name));
    });
    assert_eq!(
        found,
        BTreeSet::from([
            "flow",
            "if-let",
            "let-else",
            "match",
            "pipeline",
            "result",
            "template",
            "tt-import",
            "try-expression",
            "try-statement",
            "tuple-match",
            "val",
            "variant",
            "verbatim",
        ])
    );
}

#[test]
fn an_incomplete_try_keeps_a_structural_rollback_fact() {
    let source = "function f() {\n  const n = try g()\n  return n;\n}\n";
    let program = parse(source);
    let candidates = unclaimed_candidates(&program);
    assert_eq!(candidates.len(), 1, "{candidates:#?}");
    let candidate = candidates[0];
    assert_eq!(candidate.kind, UnclaimedTtKind::Try);
    assert_eq!(
        &source[candidate.keyword.start..candidate.keyword.end],
        "try"
    );
    assert_eq!(
        &source[candidate.extent.start..candidate.extent.end],
        "try g()"
    );
}

#[test]
fn valid_typescript_try_shapes_are_not_tt_candidates() {
    let source = "try { f(); } catch (error) { g(error); }\n\
                      const object = { try: 1 };\n\
                      object.try();\n";
    assert!(unclaimed_candidates(&parse(source)).is_empty());
}

#[test]
fn a_template_interpolation_recovery_is_collected_once() {
    let program = parse("const value = `x=${1 |> }`;\n");
    let recoveries = projection_recoveries(&program);
    assert_eq!(recoveries.len(), 1, "{recoveries:#?}");
}

#[test]
fn async_concise_arrow_claims_result() {
    let program =
        parse("const f = async (): Promise<R> => result { const x = try g(); return x; };\n");
    assert!(
        program
            .segments
            .iter()
            .any(|segment| matches!(segment, Segment::ResultBlock(_))),
        "{program:#?}"
    );
}
