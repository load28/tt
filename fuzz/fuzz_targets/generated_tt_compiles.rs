//! Programs that *are* tt: they must compile, and what comes out must be
//! TypeScript.
//!
//! The other target feeds the compiler noise and asks it not to crash.
//! This one feeds it tt — variants, matches over them with every case
//! covered, pipelines, `try`, `result` blocks, `if let` — and asks for the
//! answer to be right in the two ways a lowering can be wrong without
//! anyone noticing:
//!
//! 1. **It compiles.** A well-formed program that ttc rejects is a hole in
//!    the grammar or a rule that overreaches. The generator only builds
//!    programs the language admits, so every rejection here is a bug.
//! 2. **The emission parses as TypeScript.** `compile` runs that check
//!    itself (`--no-verify` turns it off), so this target simply leaves it
//!    on: a lowering that produces text tsc cannot read is caught at the
//!    moment it is produced rather than in someone's build.
//!
//! A structured generator rather than raw bytes, because the interesting
//! programs are vanishingly unlikely to appear by chance: libFuzzer's bytes
//! choose *shapes* here, and coverage still steers it toward the shapes
//! that reach new code.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use ttc::Options;

/// What an arm's body evaluates to. Every variant is an expression, so an
/// arm is valid wherever it appears.
#[derive(Arbitrary, Debug)]
enum Body {
    /// The payload the pattern bound, used — which is what makes the
    /// binding's lowering observable.
    Bound,
    Literal(u8),
    /// A pipeline over the bound value: two steps, so the runtime helper is
    /// needed and its import has to be placed.
    Piped,
    /// A nested match on a second variant declaration.
    Nested,
}

/// One case of a generated variant declaration: a tag, and how many payload fields.
#[derive(Arbitrary, Debug)]
struct Case {
    fields: u8,
}

/// One generated program.
#[derive(Arbitrary, Debug)]
struct Program {
    /// Two to five cases; more says nothing new and costs the fuzzer time.
    cases: Vec<Case>,
    bodies: Vec<Body>,
    /// Whether the match ends in a wildcard instead of covering every case.
    wildcard: bool,
    /// Whether the match sits inside a `result` block.
    in_result: bool,
    /// Whether the function propagates with `try`.
    with_try: bool,
    /// Whether an `if let` precedes the match.
    with_if_let: bool,
}

/// The tag of case `index` — plain ASCII, so nothing here tests the
/// scanner's UTF-8 handling by accident.
fn tag(index: usize) -> String {
    format!("C{index}")
}

fn field(case: usize, index: usize) -> String {
    format!("f{case}_{index}")
}

impl Program {
    /// The program as tt source, or `None` when the draw is degenerate.
    fn render(&self) -> Option<String> {
        let cases: Vec<usize> = self
            .cases
            .iter()
            .take(5)
            .map(|case| (case.fields % 3) as usize)
            .collect();
        if cases.len() < 2 {
            return None;
        }
        let mut out = String::new();
        out.push_str("declare function step(n: number): number;\n");
        out.push_str("declare function fallible(n: number): TResult<number, string>;\n");
        out.push_str("import type { TResult } from \"@tt/std\";\n\n");

        // The subject, plus a second variant declaration for nested arms to match on.
        out.push_str("export variant E {\n");
        for (index, fields) in cases.iter().enumerate() {
            out.push_str("  ");
            out.push_str(&tag(index));
            if *fields > 0 {
                let list: Vec<String> = (0..*fields)
                    .map(|f| format!("{}: number", field(index, f)))
                    .collect();
                out.push('(');
                out.push_str(&list.join(", "));
                out.push(')');
            }
            out.push_str(",\n");
        }
        out.push_str("}\n\nexport variant F { Yes, No }\n\n");

        out.push_str("export function run(e: E, f: F, n: number): number {\n");
        if self.with_if_let {
            // The parens are part of the pattern, not decoration: without
            // them `if let C0 = e` binds `e` to a new name called `C0`
            // instead of testing the case, which is why the language makes
            // them mandatory — a unit case is written `C0()`.
            out.push_str("  if let ");
            out.push_str(&tag(0));
            if cases[0] > 0 {
                out.push('(');
                out.push_str(&field(0, 0));
                out.push_str(") = e {\n    n = n + ");
                out.push_str(&field(0, 0));
                out.push_str(";\n  }\n");
            } else {
                out.push_str("() = e {\n    n = n + 1;\n  }\n");
            }
        }
        if self.with_try {
            out.push_str("  const checked = try fallible(n);\n  n = checked;\n");
        }

        let indent = if self.in_result { "    " } else { "  " };
        if self.in_result {
            out.push_str("  const value = result {\n    const first <- fallible(n);\n");
        }
        out.push_str(indent);
        out.push_str(if self.in_result {
            "const chosen = match (e) {\n"
        } else {
            "const chosen = match (e) {\n"
        });

        let covered = if self.wildcard { 1 } else { cases.len() };
        for (index, fields) in cases.iter().enumerate().take(covered) {
            out.push_str(indent);
            out.push_str("  ");
            out.push_str(&tag(index));
            if *fields > 0 {
                let list: Vec<String> = (0..*fields).map(|f| field(index, f)).collect();
                out.push('(');
                out.push_str(&list.join(", "));
                out.push(')');
            }
            out.push_str(" => ");
            let bound = (*fields > 0).then(|| field(index, 0));
            out.push_str(&self.body(index, bound.as_deref()));
            out.push_str(",\n");
        }
        if self.wildcard {
            out.push_str(indent);
            out.push_str("  _ => 0,\n");
        }
        out.push_str(indent);
        out.push_str("};\n");

        if self.in_result {
            out.push_str("    first + chosen\n  };\n  return value.kind === \"Ok\" ? 0 : 1;\n");
        } else {
            out.push_str("  return chosen + (f.kind === \"Yes\" ? 1 : 0);\n");
        }
        out.push_str("}\n");
        Some(out)
    }

    /// One arm's body expression.
    fn body(&self, index: usize, bound: Option<&str>) -> String {
        let choice = self.bodies.get(index % self.bodies.len().max(1));
        match (choice, bound) {
            (Some(Body::Bound), Some(name)) => name.to_string(),
            (Some(Body::Piped), Some(name)) => format!("{name} |> step |> step"),
            (Some(Body::Piped), None) => "n |> step |> step".to_string(),
            (Some(Body::Nested), _) => {
                "match (f) { Yes => 1, No => 0 }".to_string()
            }
            (Some(Body::Literal(v)), _) => format!("{}", v % 100),
            (Some(Body::Bound), None) | (None, _) => "0".to_string(),
        }
    }
}

fuzz_target!(|program: Program| {
    let Some(source) = program.render() else {
        return;
    };
    // Verification on: the emitted TypeScript has to parse. That is the
    // half of the claim a shape-generator is uniquely good at reaching.
    let options = Options::default();
    match ttc::compile(&source, &options) {
        Ok(emitted) => {
            // Deterministic: the same input twice is the same output, which
            // is what lets a build cache an emission at all.
            let again = ttc::compile(&source, &options).expect("compiled once already");
            assert_eq!(emitted, again, "compilation is not deterministic for:\n{source}");
        }
        Err(error) => panic!("a well-formed tt program was rejected: {error}\n\n{source}"),
    }
});
