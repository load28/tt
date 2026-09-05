use super::*;

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
