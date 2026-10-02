use super::*;

#[test]
fn sibling_storage_is_hygienic_and_omits_unused_subjects() {
    require_toolchain!();
    let source = r#"
export const $tt_subject = 1, $tt_subject_1 = 2, $tt_raise = 3;
declare function read(): boolean;
type Item = {run: (x: number) => number};
declare function pair(a: Item, b: Item): void;
pair(match (read()) { _ => ({run: x => x}) }, match (read()) { _ => ({run: x => x}) });
pair(match (read()) { true => ({run: x => x}), false => ({run: x => x}) }, match (read()) { true => ({run: x => x}), false => ({run: x => x}) });
"#;
    let dir = tmpdir();
    let file = dir.join("unused.ts");
    fs::write(&file, compile(source, &Options::default()).unwrap()).unwrap();
    let checked = common::tsc()
        .arg(file)
        .args(TSC_FLAGS)
        .args(["--noEmit", "--noUnusedLocals", "--noUnusedParameters"])
        .output()
        .unwrap();
    assert!(checked.status.success(), "{}", tsc_report(&checked));
}

#[test]
fn sibling_contextual_match_family_matrix() {
    require_toolchain!();
    let families = [
        "match (flag) { true => ({kind: 'item', run: x => x}), false => ({kind: 'item', run: x => x + 1}) }",
        "match (flag) { true if number > 0 => ({kind: 'item', run: x => x}), _ => ({kind: 'item', run: x => x + 1}) }",
        "match (flag) { true => { return {kind: 'item', run: x => x}; }, false => { return {kind: 'item', run: x => x + 1}; } }",
        "match (state) { Ready => ({kind: 'item', run: x => x}), Empty => ({kind: 'item', run: x => x + 1}) }",
    ];
    let dir = tmpdir();
    for kind in [SourceKind::TypeScript, SourceKind::Tsx] {
        let mut source = String::from(
            "type Item = {kind: 'item'; run: (x: number) => number};\nvariant State { Ready, Empty }\ndeclare function pair(a: Item, b: Item): void;\ndeclare function Widget(props: {a: Item; b: Item}): any;\n",
        );
        for (i, first) in families.iter().enumerate() {
            for (j, second) in families.iter().enumerate() {
                let mut hosts = vec![
                    format!("pair({first}, {second});"),
                    format!("const items: Item[] = [{first}, {second}];"),
                    format!("pair({first}, flag ? {second} : {{kind: 'item', run: x => x}});"),
                ];
                if kind == SourceKind::Tsx {
                    hosts.push(format!(
                        "const view = <Widget a={{{first}}} b={{{second}}} />;"
                    ));
                }
                for (h, host) in hosts.iter().enumerate() {
                    source.push_str(&format!("function cell_{i}_{j}_{h}(flag: boolean, number: number, state: State) {{ {host} }}\n"));
                }
            }
        }
        let file = dir.join(if kind == SourceKind::Tsx {
            "siblings.tsx"
        } else {
            "siblings.ts"
        });
        fs::write(
            &file,
            compile(
                &as_module(&source),
                &Options {
                    source_kind: kind,
                    ..Options::default()
                },
            )
            .unwrap(),
        )
        .unwrap();
        let checked = common::tsc()
            .arg(file)
            .args(TSC_FLAGS)
            .args(["--noEmit", "--jsx", "preserve"])
            .output()
            .unwrap();
        assert!(checked.status.success(), "{}", tsc_report(&checked));
    }
}

#[test]
fn guarded_contextual_values_have_no_unused_generated_locals() {
    require_toolchain!();
    let dir = tmpdir();
    let source = include_str!("../fixtures/emit/contextual-guarded-match/input.tt");
    let file = dir.join("guarded.ts");
    fs::write(
        &file,
        compile(&as_module(source), &Options::default()).unwrap(),
    )
    .unwrap();
    let checked = common::tsc()
        .arg(&file)
        .args(TSC_FLAGS)
        .args(["--noEmit", "--noUnusedLocals", "--noUnusedParameters"])
        .output()
        .unwrap();
    assert!(checked.status.success(), "{}", tsc_report(&checked));
}

#[test]
fn composed_match_values_preserve_typescript_contextual_typing() {
    require_toolchain!();
    let dir = tmpdir();
    let first = "({kind: \"item\", run: x => x + 1})";
    let second = "({kind: \"item\", run: x => x - 1})";
    let matches = [
        format!("match (flag) {{ true => {first}, false => {second} }}"),
        format!("match (number) {{ 0 | 1 => {first}, _ => {second} }}"),
        format!("match (state) {{ Ready => {first}, Empty => {second} }}"),
        format!("match (text) {{ 'ready' | 'pending' => {first}, _ => {second} }}"),
        format!("match (flag) {{ true if number > 0 => {first}, _ => {second} }}"),
        format!("match (state) {{ Ready if flag => {first}, _ => {second} }}"),
        format!(
            "match (flag) {{ true => {{ return {first}; }}, false => {{ return {second}; }} }}"
        ),
        format!(
            "match (flag) {{ true if number > 0 => {{ return {first}; }}, _ => {{ return {second}; }} }}"
        ),
    ];
    let hosts = [
        "const value: {item: Item} = {item: VALUE};",
        "const value: Item[] = [VALUE];",
        "consume(VALUE);",
        "new Container(VALUE);",
        "const value: [number, Item] = [1, VALUE];",
        "const value = (): {item: Item} => ({item: VALUE});",
        "callbackFirst(x => x + 1, VALUE);",
        "callbackFirst(function(x) { return x + 1; }, VALUE);",
    ];
    let mut files = Vec::new();
    for kind in [SourceKind::TypeScript, SourceKind::Tsx] {
        let mut source = String::from(
            "type Item = {kind: 'item'; run: (x: number) => number};\n\
             declare function consume(item: Item): void;\n\
             declare function callbackFirst(callback: (x: number) => number, item: Item): void;\n\
             declare class Container { constructor(item: Item); }\n\
             declare function Widget(props: {item: Item}): any;\n\
             variant State { Ready, Empty }\n",
        );
        let mut cells = hosts.to_vec();
        if kind == SourceKind::Tsx {
            cells.push("const view = <Widget item={VALUE} />;");
        }
        for (match_index, matched) in matches.iter().enumerate() {
            for (host_index, host) in cells.iter().enumerate() {
                source.push_str(&format!(
                    "function cell_{match_index}_{host_index}(flag: boolean, state: State, number: number, text: string) {{ {} }}\n",
                    host.replace("VALUE", matched),
                ));
                // An independent TS expression confirms the contextual host
                // and its unannotated callback are valid in the first place.
                source.push_str(&format!(
                    "function oracle_{match_index}_{host_index}(flag: boolean) {{ {} }}\n",
                    host.replace("VALUE", &format!("(flag ? {first} : {second})")),
                ));
            }
        }
        let emitted = compile(
            &as_module(&source),
            &Options {
                source_kind: kind,
                ..Options::default()
            },
        )
        .unwrap();
        let file = dir.join(if kind == SourceKind::Tsx {
            "cases.tsx"
        } else {
            "cases.ts"
        });
        fs::write(&file, emitted).unwrap();
        files.push(file);
    }
    let checked = common::tsc()
        .args(&files)
        .args(TSC_FLAGS)
        .args(["--noEmit", "--jsx", "preserve"])
        .output()
        .unwrap();
    assert!(checked.status.success(), "{}", tsc_report(&checked));
}

#[test]
fn scoped_host_call_completions_preserve_contextual_typing() {
    require_toolchain!();
    let dir = tmpdir();
    let first = "({kind: \"item\", run: x => x + value})";
    let second = "({kind: \"item\", run: x => x})";
    let matches = [
        format!("match (state) {{ Ready(value) => {first}, Empty => {second} }}"),
        format!("match (state) {{ Ready(value) if value > 0 => {first}, _ => {second} }}"),
        format!(
            "match (state) {{ Ready(value) => {{ return {first}; }}, Empty => {{ return {second}; }} }}"
        ),
        format!(
            "match (state) {{ Ready(value) => {{ const amount = value + 1; effect(); return {{kind: \"item\", run: x => x + amount}}; }}, Empty => {second} }}"
        ),
        // TASK-328: control-flow-bearing arms whose every return is free of
        // cleanup boundaries carry the call too.
        format!(
            "match (state) {{ Ready(value) => {{ if (value > 0) return {first}; return {second}; }}, Empty => {second} }}"
        ),
        format!(
            "match (state) {{ Ready(value) => {{ for (const step of [1, 2]) {{ if (step === value) return {first}; }} return {second}; }}, Empty => {second} }}"
        ),
        format!(
            "match (state) {{ Ready(value) => {{ switch (value) {{ case 0: return {second}; default: return {first}; }} }}, Empty => {second} }}"
        ),
    ];
    // TASK-327's host forms: consumed results, method and optional calls,
    // and explicit generic arguments. Every callback parameter is
    // unannotated so strict checking proves genuine contextual typing.
    let hosts = [
        "consume(VALUE);",
        "const consumed: number = consume(VALUE);",
        "return consume(VALUE);",
        "const added: number = consume(VALUE) + 1;",
        "api.consume(VALUE);",
        "const method: number = api.consume(VALUE);",
        "maybeConsume?.(VALUE);",
        "const optional: number | undefined = maybeConsume?.(VALUE);",
        "generic<Item>(VALUE);",
        "const instantiated: Item = generic<Item>(VALUE);",
    ];
    let mut files = Vec::new();
    for kind in [SourceKind::TypeScript, SourceKind::Tsx] {
        let mut source = String::from(
            "type Item = {kind: 'item'; run: (x: number) => number};\n\
             declare function consume(item: Item): number;\n\
             declare const api: { consume(item: Item): number };\n\
             declare const maybeConsume: ((item: Item) => number) | undefined;\n\
             declare function generic<T>(value: T): T;\n\
             declare function effect(): void;\n\
             variant State { Ready(value: number), Empty }\n",
        );
        for (match_index, matched) in matches.iter().enumerate() {
            for (host_index, host) in hosts.iter().enumerate() {
                source.push_str(&format!(
                    "function cell_{match_index}_{host_index}(state: State): unknown {{ {} return undefined; }}\n",
                    host.replace("VALUE", matched),
                ));
                source.push_str(&format!(
                    "function oracle_{match_index}_{host_index}(state: State, value: number): unknown {{ {} return undefined; }}\n",
                    host.replace("VALUE", &format!("(state ? {first} : {second})")),
                ));
            }
        }
        let emitted = compile(
            &as_module(&source),
            &Options {
                source_kind: kind,
                ..Options::default()
            },
        )
        .unwrap();
        let file = dir.join(if kind == SourceKind::Tsx {
            "completions.tsx"
        } else {
            "completions.ts"
        });
        fs::write(&file, emitted).unwrap();
        files.push(file);
    }
    let checked = common::tsc()
        .args(&files)
        .args(TSC_FLAGS)
        .args(["--noEmit", "--jsx", "preserve"])
        .output()
        .unwrap();
    assert!(checked.status.success(), "{}", tsc_report(&checked));
}

#[test]
fn scoped_sibling_final_arguments_keep_contextual_typing() {
    require_toolchain!();
    let dir = tmpdir();
    let scoped = [
        "match (state) { Ready(value) => ({kind: \"item\", run: x => x + value}), Empty => ({kind: \"item\", run: x => x}) }",
        "match (state) { Ready(value) if value > 0 => ({kind: \"item\", run: x => x + value}), _ => ({kind: \"item\", run: x => x}) }",
        "match (state) { Ready(value) => { const amount = value + 1; return {kind: \"item\", run: x => x + amount}; }, Empty => ({kind: \"item\", run: x => x}) }",
        "match (state) { Ready(value) => { if (value > 0) return {kind: \"item\", run: x => x + value}; return {kind: \"item\", run: x => x}; }, Empty => ({kind: \"item\", run: x => x}) }",
    ];
    // Earlier arguments are captured before the dispatch, so their own
    // types must not depend on the capture's contextual type
    // (TASK-333 tracks that separate boundary).
    let firsts = ["made", "make()", "state ? made : make()"];
    let mut files = Vec::new();
    for kind in [SourceKind::TypeScript, SourceKind::Tsx] {
        let mut source = String::from(
            "type Item = {kind: 'item'; run: (x: number) => number};\n\
             declare function pair(first: Item, second: Item): number;\n\
             declare function trio(first: Item, between: number, third: Item): number;\n\
             declare function make(): Item;\n\
             declare const made: Item;\n\
             variant State { Ready(value: number), Empty }\n",
        );
        for (scoped_index, second) in scoped.iter().enumerate() {
            for (first_index, first) in firsts.iter().enumerate() {
                source.push_str(&format!(
                    "function cell_{scoped_index}_{first_index}(state: State): number {{ return pair({first}, {second}); }}\n"
                ));
                source.push_str(&format!(
                    "function trio_{scoped_index}_{first_index}(state: State): number {{ return trio({first}, 7, {second}); }}\n"
                ));
                source.push_str(&format!(
                    "function oracle_{scoped_index}_{first_index}(state: State, value: number): number {{ return pair({first}, state ? {{kind: \"item\", run: x => x + value}} : {{kind: \"item\", run: x => x}}); }}\n"
                ));
            }
        }
        let emitted = compile(
            &as_module(&source),
            &Options {
                source_kind: kind,
                ..Options::default()
            },
        )
        .unwrap();
        let file = dir.join(if kind == SourceKind::Tsx {
            "sibling-completions.tsx"
        } else {
            "sibling-completions.ts"
        });
        fs::write(&file, emitted).unwrap();
        files.push(file);
    }
    let checked = common::tsc()
        .args(&files)
        .args(TSC_FLAGS)
        .args(["--noEmit", "--jsx", "preserve"])
        .output()
        .unwrap();
    assert!(checked.status.success(), "{}", tsc_report(&checked));
}

#[test]
fn scoped_contextual_hosts_and_cleanup_preserve_typescript_context() {
    require_toolchain!();
    let header = r#"
variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
declare const state: State; declare const flag: boolean;
declare function consume(item: Item): number;
declare function wrapped(item: {item: Item}): number;
declare function pair(first: Item, second: Item): number;
declare const api: {consume(item: Item): number};
"#;
    let value = "match (state) { Ready(value) => ({kind: \"item\", run: x => x + value}), Empty => ({kind: \"item\", run: x => x}) }";
    let native_value =
        "(() => { const value = 1; return {kind: \"item\", run: x => x + value}; })()";
    let mut failures = Vec::new();
    for (name, host) in [
        ("consumed call", "const answer = consume(VALUE);"),
        ("method call", "api.consume(VALUE);"),
        ("optional call", "consume?.(VALUE);"),
        ("object argument", "wrapped({item: VALUE});"),
        ("scoped siblings", "pair(VALUE, VALUE);"),
    ] {
        let oracle = format!("{header}{}", host.replace("VALUE", native_value));
        let (valid, report) = typecheck(&oracle);
        assert!(valid, "native oracle {name}: {report}");
        let (valid, report) = typecheck(&format!("{header}{}", host.replace("VALUE", value)));
        if !valid {
            failures.push(format!("{name}: {report}"));
        }
    }
    for (name, body) in [
        (
            "conditional returns",
            "if (flag) return {kind: \"item\", run: x => x + value}; return {kind: \"item\", run: x => x};",
        ),
        (
            "finally",
            "try { return {kind: \"item\", run: x => x + value}; } finally { console.log(\"cleanup\"); }",
        ),
        (
            "catch",
            "try { return {kind: \"item\", run: x => x + value}; } catch { return {kind: \"item\", run: x => x}; }",
        ),
    ] {
        let (valid, report) = typecheck(&format!(
            "{header}consume((() => {{ const value = 1; {body} }})());"
        ));
        assert!(valid, "native oracle {name}: {report}");
        let (valid, report) = typecheck(&format!(
            "{header}consume(match (state) {{ Ready(value) => {{ {body} }}, Empty => ({{kind: \"item\", run: x => x}}) }});"
        ));
        if !valid {
            failures.push(format!("{name}: {report}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn nested_scoped_context_preserves_disposal_and_call_order() {
    require_toolchain!();
    let lines = run_with_tsc_flags(
        r#"
type Item = {kind: "item"; run: (x: number) => number};
const events: string[] = [];
const receiver = {
  base: 10,
  consume(item: Item) { events.push("call"); return this.base + item.run(2); },
};
async function main(flag: boolean) {
  const answer = receiver.consume(match (flag) {
    true => {
      await using outer = { async [Symbol.asyncDispose]() { events.push("outer-dispose"); } };
      return match (flag) {
        true => {
          using inner = { [Symbol.dispose]() { events.push("inner-dispose"); } };
          const local = 3;
          events.push("value");
          return {kind: "item", run: x => x + local};
        },
        false => ({kind: "item", run: x => x}),
      };
    },
    false => ({kind: "item", run: x => x}),
  });
  console.log(answer, events.join(","));
}
await main(true);
"#,
        &["--lib", "es2022,dom,esnext.disposable"],
    );
    assert_eq!(lines, ["15 value,inner-dispose,outer-dispose,call"]);
}

#[test]
fn scoped_contextual_family_matrix_covers_hosts_and_nesting() {
    require_toolchain!();
    let values = [
        "match (state) { Ready(value) => ({kind: 'item', run: x => x + value}), Empty => ({kind: 'item', run: x => x}) }",
        "match (flag) { true => { const local = 1; return {kind: 'item', run: x => x + local}; }, false => ({kind: 'item', run: x => x}) }",
        "match (flag) { true => { if (flag) return {kind: 'item', run: x => x}; return {kind: 'item', run: x => x}; }, false => ({kind: 'item', run: x => x}) }",
        "match (flag) { true => { try { return {kind: 'item', run: x => x}; } catch { return {kind: 'item', run: x => x}; } }, false => ({kind: 'item', run: x => x}) }",
        "match (flag) { true => { try { return {kind: 'item', run: x => x}; } finally { console.log(flag); } }, false => ({kind: 'item', run: x => x}) }",
        "match (flag) { true => { using resource = {[Symbol.dispose]() { console.log(flag); }}; return {kind: 'item', run: x => x}; }, false => ({kind: 'item', run: x => x}) }",
        "match (flag) { true => { const local = 1; return match (flag) { true => ({kind: 'item', run: x => x + local}), false => ({kind: 'item', run: x => x}) }; }, false => ({kind: 'item', run: x => x}) }",
        "match (flag) { true => { const local = 1; return (match (flag) { true => ({kind: 'item', run: x => x + local}), false => ({kind: 'item', run: x => x}) }); }, false => ({kind: 'item', run: x => x}) }",
    ];
    let hosts = [
        "const answer = consume(VALUE);",
        "api.consume(VALUE);",
        "consume?.(VALUE);",
        "wrapped({item: VALUE});",
        "pair(VALUE, {kind: 'item', run: x => x});",
        "pair({kind: 'item', run: x => x}, VALUE);",
        "pair(VALUE, VALUE);",
        "const answer: Item = VALUE;",
    ];
    for kind in [SourceKind::TypeScript, SourceKind::Tsx] {
        let mut source = String::from(
            "export {};\ntype Item = {kind: 'item'; run: (x: number) => number};\nvariant State { Ready(value: number), Empty }\ndeclare function consume(item: Item): number;\ndeclare function wrapped(item: {item: Item}): number;\ndeclare function pair(a: Item, b: Item): number;\ndeclare const api: {consume(item: Item): number};\ndeclare function Widget(props: {item: Item}): any;\n",
        );
        for (value_index, value) in values.iter().enumerate() {
            for (host_index, host) in hosts.iter().enumerate() {
                source.push_str(&format!("export function cell_{value_index}_{host_index}(flag: boolean, state: State) {{ {} }}\n", host.replace("VALUE", value)));
            }
            if kind == SourceKind::Tsx {
                source.push_str(&format!("export function jsx_{value_index}(flag: boolean, state: State) {{ return <Widget item={{{value}}} />; }}\n"));
            }
        }
        let code = compile(
            &source,
            &Options {
                source_kind: kind,
                ..Options::default()
            },
        )
        .unwrap();
        let dir = tmpdir();
        let file = dir.join(if kind == SourceKind::Tsx {
            "matrix.tsx"
        } else {
            "matrix.ts"
        });
        fs::write(&file, code).unwrap();
        let checked = common::tsc()
            .arg(file)
            .args(TSC_FLAGS)
            .args([
                "--noEmit",
                "--jsx",
                "preserve",
                "--lib",
                "es2022,dom,esnext.disposable",
            ])
            .output()
            .unwrap();
        assert!(checked.status.success(), "{}", tsc_report(&checked));
    }
}
