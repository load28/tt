import { variant } from "./variants.mjs";

const shape = variant("Shape", [
  ["Circle", [["r", "number"]]],
  ["Rect", [["w", "number"], ["h", "number"]]],
  ["Point", null],
]);
const shapes = "[Shape.Circle(2), Shape.Rect(3, 4), Shape.Point]";

const token = variant("Token", [
  ["Num", [["value", "number"]]],
  ["Neg", [["value", "number"]]],
  ["Word", [["text", "string"]]],
  ["End", null],
]);
const tokens = '[Token.Num(4), Token.Neg(-3), Token.Word("hi"), Token.End]';

const opt = variant("Maybe", [
  ["Some", [["value", "number"]]],
  ["None", null],
]);
const outcome = variant("Outcome", [
  ["Done", [["value", "Maybe"]]],
  ["Failed", [["error", "string"]]],
]);

const speed = variant("Speed", [["Fast", null], ["Slow", null]]);

export default {
  construct: "match",
  kind: "value",
  jsx: true,
  forms: [
    {
      id: "tagBindings",
      title: "tag patterns binding payload fields",
      decls: [shape],
      In: "Shape",
      inputs: shapes,
      tt: (x) =>
        `match (${x}) { Circle(/*@bind*/r) => note("circle", /*@use*/r * 2), Rect(w, /*@bind2*/h) => /*@call*/note(/*@arg*/"rect", w * /*@use2*/h), Point => note("point", 0) }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t}.kind === "Circle" ? note("circle", ${t}.r * 2) : ${t}.kind === "Rect" ? note("rect", ${t}.w * ${t}.h) : note("point", 0))`,
      edit: (x, [t]) =>
        `((${t}: Shape) => { if (${t}.kind === "Circle") { const { /*@bind*/r } = ${t}; return note("circle", /*@use*/r * 2); } if (${t}.kind === "Rect") { const { w, /*@bind2*/h } = ${t}; return /*@call*/note(/*@arg*/"rect", w * /*@use2*/h); } return note("point", 0); })(${x})`,
    },
    {
      id: "fieldAlias",
      title: "a field bound under an alias, in any order",
      decls: [shape],
      In: "Shape",
      inputs: shapes,
      tt: (x) =>
        `match (${x}) { Rect(/*@field*/h: /*@bind*/tall, w: wide) => note("rect", [wide, /*@use*/tall]), Circle(r: /*@bind2*/radius) => note("circle", /*@use2*/radius), Point => note("point", null) }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t}.kind === "Rect" ? note("rect", [${t}.w, ${t}.h]) : ${t}.kind === "Circle" ? note("circle", ${t}.r) : note("point", null))`,
      edit: (x, [t]) =>
        `((${t}: Shape) => { if (${t}.kind === "Rect") { const { /*@field*/h: /*@bind*/tall, w: wide } = ${t}; return note("rect", [wide, /*@use*/tall]); } if (${t}.kind === "Circle") { const { r: /*@bind2*/radius } = ${t}; return note("circle", /*@use2*/radius); } return note("point", null); })(${x})`,
    },
    {
      id: "guards",
      title: "guarded arms that fall through to the next arm",
      decls: [shape],
      In: "Shape",
      inputs: "[Shape.Circle(1), Shape.Circle(5), Shape.Rect(4, 2), Shape.Rect(1, 3), Shape.Point]",
      tt: (x) =>
        `match (${x}) { Circle(/*@bind*/r) if note("guard big", /*@use*/r > 2) => note("big", /*@use2*/r), Circle(r) => note("small", r), Rect(w, h) if note("guard wide", w > h) => note("wide", w), Rect(w, h) => note("tall", h), Point => note("point", 0) }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t}.kind === "Circle" && note("guard big", ${t}.r > 2) ? note("big", ${t}.r) : ${t}.kind === "Circle" ? note("small", ${t}.r) : ${t}.kind === "Rect" && note("guard wide", ${t}.w > ${t}.h) ? note("wide", ${t}.w) : ${t}.kind === "Rect" ? note("tall", ${t}.h) : note("point", 0))`,
      edit: (x, [t]) =>
        `((${t}: Shape) => { if (${t}.kind === "Circle") { const { /*@bind*/r } = ${t}; if (note("guard big", /*@use*/r > 2)) return note("big", /*@use2*/r); } if (${t}.kind === "Circle") { const { r } = ${t}; return note("small", r); } if (${t}.kind === "Rect") { const { w, h } = ${t}; if (note("guard wide", w > h)) return note("wide", w); } if (${t}.kind === "Rect") { const { w, h } = ${t}; return note("tall", h); } return note("point", 0); })(${x})`,
    },
    {
      id: "orPattern",
      title: "an or-pattern whose alternatives bind the same field",
      decls: [token],
      In: "Token",
      inputs: tokens,
      tt: (x) =>
        `match (${x}) { Num(/*@bind*/value) | Neg(value) => note("number", /*@use*/value), Word(/*@bind2*/text) => note("word", /*@use2*/text), End => note("end", "") }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t}.kind === "Num" || ${t}.kind === "Neg" ? note("number", ${t}.value) : ${t}.kind === "Word" ? note("word", ${t}.text) : note("end", ""))`,
      edit: (x, [t]) =>
        `((${t}: Token) => { if (${t}.kind === "Num" || ${t}.kind === "Neg") { const { /*@bind*/value } = ${t}; return note("number", /*@use*/value); } if (${t}.kind === "Word") { const { /*@bind2*/text } = ${t}; return note("word", /*@use2*/text); } return note("end", ""); })(${x})`,
    },
    {
      id: "wildcard",
      title: "a final wildcard arm",
      decls: [token],
      In: "Token",
      inputs: tokens,
      tt: (x) => `match (${x}) { Word(/*@bind*/text) => note("word", /*@use*/text./*@member*/length), _ => note("other", -1) }`,
      ts: (x, [t]) => `(${t} = ${x}, ${t}.kind === "Word" ? note("word", ${t}.text.length) : note("other", -1))`,
      edit: (x, [t]) =>
        `((${t}: Token) => { if (${t}.kind === "Word") { const { /*@bind*/text } = ${t}; return note("word", /*@use*/text./*@member*/length); } return note("other", -1); })(${x})`,
    },
    {
      id: "nestedPattern",
      title: "nested patterns over a payload that is a variant",
      decls: [opt, outcome],
      In: "Outcome",
      inputs: "[Outcome.Done(Maybe.Some(7)), Outcome.Done(Maybe.None), Outcome.Failed(\"lost\")]",
      tt: (x) =>
        `match (${x}) { Done(value: Some(value: /*@bind*/v)) => note("some", /*@use*/v), Done(value: None()) => note("none", 0), Failed(/*@bind2*/error) => note("failed", /*@use2*/error) }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t}.kind === "Done" && ${t}.value.kind === "Some" ? note("some", ${t}.value.value) : ${t}.kind === "Done" && ${t}.value.kind === "None" ? note("none", 0) : note("failed", ${t}.error))`,
      edit: (x, [t]) =>
        `((${t}: Outcome) => { if (${t}.kind === "Done" && ${t}.value.kind === "Some") { const /*@bind*/v = ${t}.value.value; return note("some", /*@use*/v); } if (${t}.kind === "Done") { return note("none", 0); } { const { /*@bind2*/error } = ${t}; return note("failed", /*@use2*/error); } })(${x})`,
    },
    {
      id: "stringLiterals",
      title: "string literal patterns with an or-pattern",
      In: "string",
      inputs: '["north", "south", "east", "up"]',
      tt: (x) => `match (${x}) { "north" => /*@call*/note(/*@arg*/"n", 1), "south" | "east" => note("se", 2), _ => note("other", 0) }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t} === "north" ? /*@call*/note(/*@arg*/"n", 1) : ${t} === "south" || ${t} === "east" ? note("se", 2) : note("other", 0))`,
    },
    {
      id: "numberLiterals",
      title: "number literal patterns written in several notations, with a guard",
      In: "number",
      inputs: "[200, 201, 404, 500, 7]",
      tt: (x) =>
        `match (${x}) { 200 | 0xc9 => note("success", true), 404 => note("missing", false), 500 if /*@call*/note(/*@arg*/"guard", true) => note("server", false), _ => note("other", null) }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t} === 200 || ${t} === 201 ? note("success", true) : ${t} === 404 ? note("missing", false) : ${t} === 500 && /*@call*/note(/*@arg*/"guard", true) ? note("server", false) : note("other", null))`,
    },
    {
      id: "booleanLiterals",
      title: "boolean literal patterns without a wildcard",
      In: "boolean",
      inputs: "[true, false]",
      tt: (x) => `match (${x}) { true => note("yes", 1), false => /*@call*/note(/*@arg*/"no", 0) }`,
      ts: (x, [t]) => `(${t} = ${x}, ${t} === true ? note("yes", 1) : /*@call*/note(/*@arg*/"no", 0))`,
    },
    {
      id: "tuple",
      title: "a tuple match over two scrutinees",
      decls: [shape, speed],
      In: "Shape",
      inputs: shapes,
      tt: (x) =>
        `match (${x}, note<Speed>("second", Speed./*@member*/Fast)) { (Circle(/*@bind*/r), Fast) => note("fast circle", /*@use*/r), (Rect(/*@bind2*/w), _) => note("rect", /*@use2*/w), (_, Slow) => note("slow", 0), (Circle | Point, _) => note("rest", -1) }`,
      ts: (x, [t, u]) =>
        `(${t} = ${x}, ${u} = note<Speed>("second", Speed.Fast), ${t}.kind === "Circle" && ${u}.kind === "Fast" ? note("fast circle", ${t}.r) : ${t}.kind === "Rect" ? note("rect", ${t}.w) : ${u}.kind === "Slow" ? note("slow", 0) : note("rest", -1))`,
      edit: (x, [t, u]) =>
        `((${t}: Shape, ${u}: Speed) => { if (${t}.kind === "Circle" && ${u}.kind === "Fast") { const { /*@bind*/r } = ${t}; return note("fast circle", /*@use*/r); } if (${t}.kind === "Rect") { const { /*@bind2*/w } = ${t}; return note("rect", /*@use2*/w); } if (${u}.kind === "Slow") return note("slow", 0); return note("rest", -1); })(${x}, note<Speed>("second", Speed./*@member*/Fast))`,
      temps: 2,
    },
    {
      id: "isPatterns",
      title: "`is` patterns over error classes",
      In: "unknown",
      inputs: '[new RangeError("range"), new TypeError("type"), new Error("plain"), "text"]',
      tt: (x) =>
        `match (${x}) { is /*@call*/RangeError { /*@bind*/message } => note("range", /*@use*/message), is TypeError | is SyntaxError => note("type", 1), is Error { /*@field*/message: /*@bind2*/detail } => note("error", /*@use2*/detail), _ => note("other", null) }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t} instanceof RangeError ? note("range", ${t}.message) : ${t} instanceof TypeError || ${t} instanceof SyntaxError ? note("type", 1) : ${t} instanceof Error ? note("error", ${t}.message) : note("other", null))`,
      edit: (x, [t]) =>
        `((${t}: unknown) => { if (${t} instanceof /*@call*/RangeError) { const { /*@bind*/message } = ${t}; return note("range", /*@use*/message); } if (${t} instanceof TypeError || ${t} instanceof SyntaxError) return note("type", 1); if (${t} instanceof Error) { const { /*@field*/message: /*@bind2*/detail } = ${t}; return note("error", /*@use2*/detail); } return note("other", null); })(${x})`,
    },
    {
      id: "blockArm",
      title: "a block arm whose return is the arm's value",
      decls: [shape],
      In: "Shape",
      inputs: shapes,
      tt: (x) =>
        `match (${x}) { Rect(/*@bind*/w, h) => { const /*@bind2*/area = note("area", /*@use*/w * h); if (/*@use2*/area > 10) { return note("large", area); } return note("small", area); }, _ => note("other", 0) }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t}.kind === "Rect" ? (() => { const area = note("area", ${t}.w * ${t}.h); if (area > 10) { return note("large", area); } return note("small", area); })() : note("other", 0))`,
      edit: (x, [t]) =>
        `((${t}: Shape) => { if (${t}.kind === "Rect") { const { /*@bind*/w, h } = ${t}; const /*@bind2*/area = note("area", /*@use*/w * h); if (/*@use2*/area > 10) { return note("large", area); } return note("small", area); } return note("other", 0); })(${x})`,
    },
    {
      id: "handWrittenUnion",
      title: "tag patterns over a hand-written kind union",
      decls: [
        {
          tt: 'type Event = { kind: "Click"; x: number } | { kind: "Key"; key: string };',
          ts: 'type Event = { kind: "Click"; x: number } | { kind: "Key"; key: string };',
        },
      ],
      In: "Event",
      inputs: '[{ kind: "Click", x: 3 }, { kind: "Key", key: "k" }] as Event[]',
      tt: (x) => `match (${x}) { Click(/*@bind*/x) => note("click", /*@use*/x), Key(/*@bind2*/key) => note("key", /*@use2*/key) }`,
      ts: (x, [t]) => `(${t} = ${x}, ${t}.kind === "Click" ? note("click", ${t}.x) : note("key", ${t}.key))`,
      edit: (x, [t]) =>
        `((${t}: Event) => { if (${t}.kind === "Click") { const { /*@bind*/x } = ${t}; return note("click", /*@use*/x); } { const { /*@bind2*/key } = ${t}; return note("key", /*@use2*/key); } })(${x})`,
    },
    {
      id: "jsxArms",
      title: "arms that render different elements",
      surfaces: ["ttx"],
      decls: [shape],
      In: "Shape",
      inputs: shapes,
      tt: (x) =>
        `match (${x}) { Circle(/*@bind*/r) => <b>{note("circle", /*@use*/r)}</b>, Rect(/*@bind2*/w, h) => <i /*@attr*/title={String(/*@use2*/w)}>{h}</i>, Point => <>point</> }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t}.kind === "Circle" ? <b>{note("circle", ${t}.r)}</b> : ${t}.kind === "Rect" ? <i title={String(${t}.w)}>{${t}.h}</i> : <>point</>)`,
      edit: (x, [t]) =>
        `((${t}: Shape) => { if (${t}.kind === "Circle") { const { /*@bind*/r } = ${t}; return <b>{note("circle", /*@use*/r)}</b>; } if (${t}.kind === "Rect") { const { /*@bind2*/w, h } = ${t}; return <i /*@attr*/title={String(/*@use2*/w)}>{h}</i>; } return <>point</>; })(${x})`,
    },
    {
      id: "nestedMatch",
      title: "a match whose scrutinee and arm are matches",
      decls: [shape],
      In: "Shape",
      inputs: shapes,
      tt: (x) =>
        `match (match (${x}) { Point => note("inner point", 0), Circle(/*@bind*/r) => note("inner circle", /*@use*/r), Rect(w) => note("inner rect", w) }) { 0 => note("zero", "none"), _ => match (note("again", Shape./*@member*/Point)) { Point => note("point", "some"), _ => note("other", "?") } }`,
      ts: (x, [t, u, v]) =>
        `(${u} = (${t} = ${x}, ${t}.kind === "Point" ? note("inner point", 0) : ${t}.kind === "Circle" ? note("inner circle", ${t}.r) : note("inner rect", ${t}.w)), ${u} === 0 ? note("zero", "none") : (${v} = note("again", Shape.Point), ${v}.kind === "Point" ? note("point", "some") : note("other", "?")))`,
      edit: (x, [t, u, v]) =>
        `((${u}: number) => { if (${u} === 0) return note("zero", "none"); return ((${v}: Shape) => { if (${v}.kind === "Point") return note("point", "some"); return note("other", "?"); })(note("again", Shape./*@member*/Point)); })(((${t}: Shape) => { if (${t}.kind === "Point") return note("inner point", 0); if (${t}.kind === "Circle") { const { /*@bind*/r } = ${t}; return note("inner circle", /*@use*/r); } { const { w } = ${t}; return note("inner rect", w); } })(${x}))`,
      temps: 3,
    },
  ],
};
