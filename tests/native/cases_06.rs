fn assert_type_checks(source: &str) {
    let dir = project(&[("src/main.tt", source)]);
    let output = run(&dir, &["--check-types", "src"]);
    assert!(
        output.status.success(),
        "{source}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn each_pipeline_step_holds_the_type_of_its_own_value() {
    require_tsgo!();
    assert_type_checks(
        "declare const flag: boolean;\n\
declare function pick(value: { kind: \"a\" } | { kind: \"b\" }): string;\n\
export const a = match (1) { _ => [1] } |> (p => p.length);\n\
export const b: string = match (flag) { true => [1, 2], false => [3] } |> (p => p.length) |> String;\n\
export const c: number = match (flag) { true => \"xy\", false => \"z\" } |> .length |> (n => [n]) |> .length;\n\
export const d = match (flag) { true => ({ kind: \"a\" }), false => ({ kind: \"b\" }) } |> pick;\n\
export const e: string[] = [match (flag) { true => 1, false => 2 }] |> .map(n => n + 1) |> .map(String);\n\
export function f(): string { return match (flag) { true => [1], false => [] } |> .length |> String; }\n",
    );
}

#[test]
fn a_hoisted_value_in_a_member_step_type_checks() {
    require_tsgo!();
    assert_type_checks(
        "class Box {\n  constructor(readonly n: number) {}\n  add(amount: number): Box { return new Box(this.n + amount); }\n}\n\
declare const flag: boolean;\n\
export const a: number = new Box(1) |> .add(match (flag) { true => 1, false => 2 }) |> .n;\n\
export const b: number = new Box(1) |> .add(1).add(match (flag) { true => 1, false => 2 }).n;\n\
export const c: number = { list: [1, 2] } |> .list[match (flag) { true => 0, false => 1 }];\n\
export const d: Box | undefined = (new Box(1) as Box | undefined) |> ?.add(match (flag) { true => 1, false => 2 });\n",
    );
}

#[test]
fn a_try_in_a_template_in_a_pipeline_type_checks() {
    require_tsgo!();
    assert_type_checks(
        "type R = { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\n\
type S = { kind: \"Ok\"; value: string } | { kind: \"Err\"; error: string };\n\
declare function read(): R;\n\
declare function wrap(value: number): string;\n\
export function head(): S {\n  const value = `${wrap(try read())}!` |> String;\n  return { kind: \"Ok\", value };\n}\n\
export function step(): S {\n  const value = \"v\" |> ((tail: string) => (v: string) => v + tail)(`${wrap(try read())}`);\n  return { kind: \"Ok\", value };\n}\n",
    );
}
