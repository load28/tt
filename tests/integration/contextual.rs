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
    let checked = Command::new("tsc")
        .arg(file)
        .args(TSC_FLAGS)
        .args(["--noEmit", "--noUnusedLocals", "--noUnusedParameters"])
        .output()
        .unwrap();
    assert!(checked.status.success(), "{}", tsc_report(&checked));
}

#[test]
fn scoped_call_completions_preserve_context_scope_and_effects() {
    require_toolchain!();
    let output = run(r#"
variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
const trace: string[] = [];
function consume(item: Item) { trace.push("call:" + item.run(3)); }
for (const state of [State.Ready(4), State.Empty]) {
  trace.length = 0;
  consume(match (state) {
    Ready(value) => {
      const consume = value + 1;
      const local = () => consume;
      trace.push("arm");
      return {kind: "item", run: x => x + local()};
    },
    Empty => ({kind: "item", run: x => x - 1}),
  });
  trace.push("after");
  console.log(trace.join(","));
}
function throws(item: Item): void { trace.push("throws:" + item.run(2)); throw new Error("consumer"); }
trace.length = 0;
try {
  throws(match (State.Ready(5)) {
    Ready(value) => { trace.push("value"); return {kind: "item", run: x => x + value}; },
    Empty => ({kind: "item", run: x => x}),
  });
} catch { trace.push("caught"); }
console.log(trace.join(","));
"#);
    assert_eq!(
        output,
        ["arm,call:8,after", "call:2,after", "value,throws:7,caught"]
    );
}

#[test]
fn scoped_contextual_call_variants() {
    require_toolchain!();
    let (valid, output) = typecheck(
        r#"
variant State { Ready(value: number), Empty }
declare const state: State;
declare const flag: boolean;
declare function consume(item: {kind: "item"; run: (x: number) => number}): void;
consume(match (state) {
  Ready(value) if value > 0 => ({kind: "item", run: x => x + value}),
  _ => ({kind: "item", run: x => x}),
});
consume(match (state) {
  Ready(value: amount) => ({kind: "item", run: x => x + amount}),
  Empty => ({kind: "item", run: x => x}),
});
consume(match (flag) {
  true => { const amount = 1; return {kind: "item", run: x => x + amount}; },
  false => { function amount() { return 2; } return {kind: "item", run: x => x + amount()}; },
});
"#,
    );
    assert!(valid, "{output}");
}

#[test]
fn inline_sibling_failure_keeps_abrupt_completion() {
    require_toolchain!();
    let output = run(r#"
const trace: string[] = [];
function bad(): boolean { return JSON.parse('"bad"'); }
function pair(a: number, b: number) { trace.push("call"); }
try {
  pair(match (bad()) { true => 1, false => 2 }, match (true) { true => (trace.push("second"), 3), _ => 4 });
} catch (error) {
  console.log(error instanceof Error && error.message.includes("unexpected literal"), trace.length);
}
"#);
    assert_eq!(output, ["true 0"]);
}

#[test]
fn sibling_matches_preserve_context_and_native_argument_order() {
    require_toolchain!();
    let output = run(r#"
const trace: string[] = [];
type Item = {kind: "item"; run: (x: number) => number};
function mark<T>(name: string, value: T): T { trace.push(name); return value; }
let flag: boolean = true;
const receiver = {
  base: 10,
  get pair() {
    trace.push("callee");
    return function(this: {base: number}, a: Item, between: number, b: Item) {
      trace.push("call"); return this.base + a.run(between) + b.run(2);
    };
  },
};
const result = mark("receiver", receiver).pair(
  match (mark("subject1", flag)) {
    true => (trace.push("arm1"), flag = false, {kind: "item", run: x => x + 1}),
    false => ({kind: "item", run: x => x}),
  },
  mark("between", 3),
  match (mark("subject2", flag)) {
    true => ({kind: "item", run: x => x}),
    false => { return (trace.push("arm2"), {kind: "item", run: x => x + 2}); },
  },
);
console.log(result, trace.join(","));
"#);
    assert_eq!(
        output,
        ["18 receiver,callee,subject1,arm1,between,subject2,arm2,call"]
    );
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
        let checked = Command::new("tsc")
            .arg(file)
            .args(TSC_FLAGS)
            .args(["--noEmit", "--jsx", "preserve"])
            .output()
            .unwrap();
        assert!(checked.status.success(), "{}", tsc_report(&checked));
    }
}

#[test]
fn single_return_match_blocks_keep_context_and_runtime_order() {
    require_toolchain!();
    let output = run(r#"
const trace: string[] = [];
const mark = <T,>(name: string, value: T): T => { trace.push(name); return value; };
const receiver = {
  base: 10,
  consume(first: number, item: {kind: "item"; run: (x: number) => number}, last: number) {
    trace.push("call");
    return this.base + first + item.run(last);
  },
};
for (const flag of [true, false]) {
  trace.length = 0;
  const answer = mark("receiver", receiver).consume(mark("first", 2), match (mark("subject", flag)) {
    true => { /* selected block */ return (mark("yes", 0), {kind: "item", run: x => x + 1}); /* tail */ },
    false => { return (mark("no", 0), {kind: "item", run: x => x - 1}); },
  }, mark("last", 3));
  console.log(answer, trace.join(","));
}
"#);
    assert_eq!(
        output,
        [
            "16 receiver,first,subject,yes,last,call",
            "14 receiver,first,subject,no,last,call",
        ]
    );
}

#[test]
fn statementful_match_blocks_keep_effects_and_scope() {
    require_toolchain!();
    let output = run(r#"
const trace: string[] = [];
const label = 9;
function consume(value: number) { trace.push("call"); return value; }
for (const flag of [true, false]) {
  trace.length = 0;
  const result = consume(match (flag) {
    true => { const label = 3; trace.push("before"); return label; },
    false => { try { return label; } finally { trace.push("finally"); } },
  });
  console.log(result, label, trace.join(","));
}
"#);
    assert_eq!(output, ["3 9 before,call", "9 9 finally,call"]);
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
    let checked = Command::new("tsc")
        .arg(&file)
        .args(TSC_FLAGS)
        .args(["--noEmit", "--noUnusedLocals", "--noUnusedParameters"])
        .output()
        .unwrap();
    assert!(checked.status.success(), "{}", tsc_report(&checked));
}

#[test]
fn match_guards_keep_their_type_narrowing_scope() {
    require_toolchain!();
    let (valid, output) = typecheck(
        r#"
declare const input: unknown;
declare const flag: boolean;
declare function consume(value: number): void;
consume(match (flag) {
  true if typeof input === "string" => input.length,
  _ => 0,
});
"#,
    );
    assert!(valid, "{output}");
}

#[test]
fn composed_match_literals_keep_narrow_contexts() {
    require_toolchain!();
    let (valid, output) = typecheck(
        r#"
declare const flag: boolean;
declare function stringValue(value: "one" | "two"): void;
declare function numberValue(value: 1 | 2): void;
stringValue(match (flag) { true => "one", false => "two" });
numberValue(match (flag) { true => 1, false => 2 });
"#,
    );
    assert!(valid, "{output}");
}

#[test]
fn contextual_match_still_rejects_incompatible_callbacks() {
    require_toolchain!();
    let (valid, output) = typecheck(
        r#"
declare const flag: boolean;
declare function consume(item: {run: (x: number) => number}): void;
consume(match (flag) {
  true => ({run: (x: string) => x.length}),
  false => ({run: x => x + 1}),
});
"#,
    );
    assert!(!valid, "incompatible callback compiled: {output}");
    assert!(
        output.contains("string") && output.contains("number"),
        "{output}"
    );
}

#[test]
fn uncaptured_function_creation_keeps_defaults_and_bodies_lazy() {
    require_toolchain!();
    let output = run(r#"
const trace: string[] = [];
const mark = <T,>(name: string, value: T): T => { trace.push(name); return value; };
function consume(callback: (value?: number) => number, item: {run: (x: number) => number}) {
  trace.push("call");
  return callback() + item.run(2);
}
const first = consume((value = mark("default", 5)) => { trace.push("body"); return value; }, match (mark<boolean>("subject", true)) {
  true => (trace.push("arm"), {run: x => x + 1}),
  false => ({run: x => x - 1}),
});
console.log(first, trace.join(","));
trace.length = 0;
const second = consume(function(value = mark("default", 5)) { trace.push("body"); return value; }, match (mark<boolean>("subject", false)) {
  true => ({run: x => x + 1}),
  false => (trace.push("arm"), {run: x => x - 1}),
});
console.log(second, trace.join(","));
"#);
    assert_eq!(
        output,
        [
            "8 subject,arm,call,default,body",
            "6 subject,arm,call,default,body"
        ]
    );
}

#[test]
fn composed_match_still_throws_for_an_unexpected_runtime_value() {
    require_toolchain!();
    let output = run(r#"
function choose(flag: boolean) {
  return [match (flag) { true => 1, false => 2 }];
}
try {
  choose("invalid" as unknown as boolean);
} catch (error) {
  console.log(error instanceof Error ? error.message : String(error));
}
"#);
    assert_eq!(output, ["tt match: unexpected literal \"invalid\""]);
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
    let checked = Command::new("tsc")
        .args(&files)
        .args(TSC_FLAGS)
        .args(["--noEmit", "--jsx", "preserve"])
        .output()
        .unwrap();
    assert!(checked.status.success(), "{}", tsc_report(&checked));
}

#[test]
fn deferred_match_values_preserve_order_and_receiver() {
    require_toolchain!();
    let output = run(r#"
const trace: string[] = [];
const mark = <T,>(name: string, value: T): T => { trace.push(name); return value; };
const receiver = {
  value: 10,
  consume(first: number, item: {kind: "item"; run: (x: number) => number}, last: number) {
    trace.push("call");
    return this.value + first + item.run(last);
  },
};
for (const flag of [true, false]) {
  trace.length = 0;
  const value = mark("receiver", receiver).consume(mark("first", 2), match (mark("subject", flag)) {
    true => (trace.push("yes"), {kind: "item", run: x => x + 1}),
    _ => (trace.push("no"), {kind: "item", run: x => x - 1}),
  }, mark("last", 3));
  console.log(value, trace.join(","));
}
"#);
    assert_eq!(
        output,
        [
            "16 receiver,first,subject,yes,last,call",
            "14 receiver,first,subject,no,last,call",
        ]
    );
}

#[test]
fn guarded_contextual_values_preserve_short_circuiting_and_abrupt_completion() {
    require_toolchain!();
    let output = run(r#"
const trace: string[] = [];
const mark = <T,>(name: string, value: T): T => { trace.push(name); return value; };
const receiver = {
  value: 10,
  consume(first: number, item: {kind: "item"; run: (x: number) => number}, last: number) {
    trace.push("call");
    return this.value + first + item.run(last);
  },
};
function guard(value: number): boolean {
  trace.push("guard");
  if (value === 2) throw new Error("guard failed");
  return value === 1;
}
for (const value of [0, 1, 2, 3]) {
  trace.length = 0;
  try {
    const result = mark("receiver", receiver).consume(mark("first", 2), match (mark("subject", value < 3)) {
      true if guard(value) => (trace.push("yes"), {kind: "item", run: x => x + 1}),
      true if mark("second guard", true) => (trace.push("second"), {kind: "item", run: x => x}),
      _ => (trace.push("no"), {kind: "item", run: x => x - 1}),
    }, mark("last", 3));
    console.log(result, trace.join(","));
  } catch {
    console.log("thrown", trace.join(","));
  }
}
"#);
    assert_eq!(
        output,
        [
            "15 receiver,first,subject,guard,second guard,second,last,call",
            "16 receiver,first,subject,guard,yes,last,call",
            "thrown receiver,first,subject,guard",
            "14 receiver,first,subject,no,last,call",
        ]
    );
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
    let checked = Command::new("tsc")
        .args(&files)
        .args(TSC_FLAGS)
        .args(["--noEmit", "--jsx", "preserve"])
        .output()
        .unwrap();
    assert!(checked.status.success(), "{}", tsc_report(&checked));
}

#[test]
fn consumed_call_completions_preserve_results_order_and_errors() {
    require_toolchain!();
    let output = run(r#"
variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
const trace: string[] = [];
function mark<T>(name: string, value: T): T { trace.push(name); return value; }
const api = {
  base: 10,
  get consume() {
    trace.push("callee");
    return function(this: {base: number}, item: Item) { trace.push("call"); return this.base + item.run(3); };
  },
};
for (const state of [State.Ready(4), State.Empty]) {
  trace.length = 0;
  const result = mark("receiver", api).consume(match (mark("subject", state)) {
    Ready(value) => (trace.push("arm"), {kind: "item", run: x => x + value}),
    Empty => ({kind: "item", run: x => x}),
  }) + mark("after", 1);
  console.log(result, trace.join(","));
}
function throws(item: Item): number { trace.push("throws:" + item.run(1)); throw new Error("consumer"); }
trace.length = 0;
let unreached = "kept";
try {
  unreached = "lost:" + throws(match (State.Ready(2)) {
    Ready(value) => { trace.push("value"); return {kind: "item", run: x => x + value}; },
    Empty => ({kind: "item", run: x => x}),
  });
} catch { trace.push("caught"); }
console.log(unreached, trace.join(","));
"#);
    assert_eq!(
        output,
        [
            "18 receiver,callee,subject,arm,call,after",
            "14 receiver,callee,subject,call,after",
            "kept value,throws:3,caught",
        ]
    );
}

#[test]
fn optional_call_completions_skip_subjects_and_deliver_results() {
    require_toolchain!();
    let output = run(r#"
variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
const trace: string[] = [];
function mark<T>(name: string, value: T): T { trace.push(name); return value; }
const present: ((item: Item) => number) | undefined = item => { trace.push("call"); return item.run(2); };
const absent: ((item: Item) => number) | undefined = undefined;
for (const consume of [present, absent]) {
  trace.length = 0;
  const result = consume?.(match (mark("subject", State.Ready(5))) {
    Ready(value) => (trace.push("arm"), {kind: "item", run: x => x + value}),
    Empty => ({kind: "item", run: x => x}),
  });
  console.log(result, trace.join(",") || "silent");
}
const holder = {
  base: 100,
  maybe(item: Item): number { trace.push("method"); return this.base + item.run(1); },
};
trace.length = 0;
const viaReceiver = holder.maybe?.(match (mark("subject", State.Ready(7))) {
  Ready(value) => ({kind: "item", run: x => x * value}),
  Empty => ({kind: "item", run: x => x}),
});
console.log(viaReceiver, trace.join(","));
"#);
    assert_eq!(
        output,
        [
            "7 subject,arm,call",
            "undefined silent",
            "107 subject,method"
        ]
    );
}

#[test]
fn instantiated_call_completions_keep_generic_context() {
    require_toolchain!();
    let output = run(r#"
variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
const trace: string[] = [];
function generic<T>(value: T): T { trace.push("call"); return value; }
const item = generic<Item>(match (State.Ready(3)) {
  Ready(value) => { trace.push("arm"); return {kind: "item", run: x => x + value}; },
  Empty => ({kind: "item", run: x => x}),
});
console.log(item.run(1), trace.join(","));
"#);
    assert_eq!(output, ["4 arm,call"]);
}

#[test]
fn control_flow_arm_completions_call_at_the_authored_exit() {
    require_toolchain!();
    let output = run(r#"
variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
const trace: string[] = [];
function mark<T>(name: string, value: T): T { trace.push(name); return value; }
function consume(item: Item): number { trace.push("call"); return item.run(2); }
for (const state of [State.Ready(1), State.Ready(5), State.Empty]) {
  trace.length = 0;
  const result = consume(match (mark("subject", state)) {
    Ready(value) => {
      for (const step of [1, 2]) {
        if (step === value) { trace.push("loop"); return {kind: "item", run: x => x + value}; }
      }
      trace.push("fallthrough");
      return {kind: "item", run: x => x - value};
    },
    Empty => ({kind: "item", run: x => x}),
  }) + mark("after", 10);
  console.log(result, trace.join(","));
}
function throwing(item: Item): number { trace.push("throwing"); throw new Error("consumer"); }
trace.length = 0;
try {
  throwing(match (State.Ready(1)) {
    Ready(value) => { if (value > 0) return {kind: "item", run: x => x}; trace.push("unreached"); return {kind: "item", run: x => x}; },
    Empty => ({kind: "item", run: x => x}),
  });
} catch { trace.push("caught"); }
console.log(trace.join(","));
"#);
    assert_eq!(
        output,
        [
            "13 subject,loop,call,after",
            "7 subject,fallthrough,call,after",
            "12 subject,call,after",
            "throwing,caught",
        ]
    );
}

#[test]
fn cleanup_bearing_arms_keep_the_consumer_outside_the_arm() {
    require_toolchain!();
    let output = run(r#"
variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
const trace: string[] = [];
function consume(item: Item): number { trace.push("call"); return item.run(1); }
const finalized = consume(match (State.Ready(3)) {
  Ready(value) => {
    try {
      trace.push("arm");
      return {kind: "item" as const, run: (x: number) => x + value};
    } finally {
      trace.push("finally");
    }
  },
  Empty => ({kind: "item" as const, run: (x: number) => x}),
});
console.log(finalized, trace.join(","));
trace.length = 0;
function throwingConsumer(item: Item): number { trace.push("throwing"); throw new Error("consumer"); }
try {
  throwingConsumer(match (State.Ready(2)) {
    Ready(value) => {
      try {
        return {kind: "item" as const, run: (x: number) => x + value};
      } catch {
        trace.push("handler");
        return {kind: "item" as const, run: (x: number) => x};
      }
    },
    Empty => ({kind: "item" as const, run: (x: number) => x}),
  });
} catch { trace.push("caught outside"); }
console.log(trace.join(","));
"#);
    assert_eq!(output, ["4 arm,finally,call", "throwing,caught outside"]);
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
    let checked = Command::new("tsc")
        .args(&files)
        .args(TSC_FLAGS)
        .args(["--noEmit", "--jsx", "preserve"])
        .output()
        .unwrap();
    assert!(checked.status.success(), "{}", tsc_report(&checked));
}

#[test]
fn scoped_sibling_completions_preserve_order_and_slots() {
    require_toolchain!();
    let output = run(r#"
variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
const trace: string[] = [];
function mark<T>(name: string, value: T): T { trace.push(name); return value; }
function pair(first: Item, second: Item): number { trace.push("call"); return first.run(1) + second.run(2); }
let shared = 10;
for (const state of [State.Ready(4), State.Empty]) {
  trace.length = 0;
  shared = 10;
  const result = pair(
    match (mark("subject1", state)) {
      Ready(value) => (trace.push("arm1"), shared = value, {kind: "item" as const, run: (x: number) => x + shared}),
      Empty => ({kind: "item" as const, run: (x: number) => x}),
    },
    match (mark("subject2", shared > 5)) {
      true => (trace.push("arm2:big"), {kind: "item", run: x => x * shared}),
      false => (trace.push("arm2:small"), {kind: "item", run: x => x + shared}),
    },
  );
  console.log(result, trace.join(","));
}
function throwingSubject(): State { trace.push("boom"); throw new Error("subject"); }
trace.length = 0;
try {
  pair(
    match (State.Ready(1)) { Ready(value) => (trace.push("first"), {kind: "item" as const, run: (x: number) => x + value}), Empty => ({kind: "item" as const, run: (x: number) => x}) },
    match (throwingSubject()) { Ready(value) => ({kind: "item", run: x => x + value}), Empty => ({kind: "item", run: x => x}) },
  );
} catch { trace.push("caught"); }
console.log(trace.join(","));
"#);
    assert_eq!(
        output,
        [
            "11 subject1,arm1,subject2,arm2:small,call",
            "21 subject1,subject2,arm2:big,call",
            "first,boom,caught",
        ]
    );
}

#[test]
fn inert_literal_arguments_keep_their_contextual_position() {
    require_toolchain!();
    let (valid, output) = typecheck(
        r#"
variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
declare const state: State;
declare function widget(props: {first: Item; second: Item}): number;
declare function tagged(first: Item, second: number[], third: Item): number;
const items: Item[] = [{kind: "item", run: x => x}, made];
const nested: {first: Item; second: Item} = {
  first: {kind: "item", run: x => x},
  second: made,
};
declare const made: Item;
const wrapped = widget({
  first: {kind: "item", run: x => x},
  second: match (state) {
    Ready(value) => (made),
    Empty => (made),
  },
});
const counted = tagged({kind: "item", run: x => x}, [1, 2], made);
export {items, nested, wrapped, counted};
"#,
    );
    assert!(valid, "{output}");
}

#[test]
fn elided_and_captured_inputs_preserve_order_identity_and_bindings() {
    require_toolchain!();
    let output = run(r#"
variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: () => number};
const trace: string[] = [];
function mark<T>(name: string, value: T): T { trace.push(name); return value; }
let shared = 1;
for (const state of [State.Ready(9), State.Empty]) {
  trace.length = 0;
  shared = 1;
  const items: Item[] = [{kind: "item", run: () => shared}, match (mark("subject", state)) {
    Ready(value) => (trace.push("arm"), shared = value, {kind: "item" as const, run: () => shared + 1}),
    Empty => ({kind: "item" as const, run: () => 0}),
  }];
  console.log(`${items[0].run()}:${items[1].run()}:${items[0] === items[0]}`, trace.join(","));
}
function pair(first: Item, second: Item): string { trace.push("call"); return `${first.run()}:${second.run()}`; }
trace.length = 0;
shared = 1;
const completed = pair({kind: "item" as const, run: () => shared}, match (mark("subject", State.Ready(4))) {
  Ready(value) => (trace.push("arm"), shared = value, {kind: "item", run: () => shared + 1}),
  Empty => ({kind: "item", run: () => 0}),
});
console.log(completed, trace.join(","));
trace.length = 0;
function effectful(): Item { trace.push("effectful"); return {kind: "item", run: () => 0}; }
pair(effectful(), match (mark<State>("subject", State.Empty)) {
  Ready(value) => ({kind: "item", run: () => value}),
  Empty => ({kind: "item", run: () => 0}),
});
console.log(trace.join(","));
"#);
    assert_eq!(
        output,
        [
            "9:10:true subject,arm",
            "1:0:true subject",
            "4:5 subject,arm,call",
            "effectful,subject,call",
        ]
    );
}
