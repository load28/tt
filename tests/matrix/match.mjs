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
  forms: [
    {
      id: "tagBindings",
      title: "tag patterns binding payload fields",
      decls: [shape],
      In: "Shape",
      inputs: shapes,
      tt: (x) =>
        `match (${x}) { Circle(r) => note("circle", r * 2), Rect(w, h) => note("rect", w * h), Point => note("point", 0) }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t}.kind === "Circle" ? note("circle", ${t}.r * 2) : ${t}.kind === "Rect" ? note("rect", ${t}.w * ${t}.h) : note("point", 0))`,
    },
    {
      id: "fieldAlias",
      title: "a field bound under an alias, in any order",
      decls: [shape],
      In: "Shape",
      inputs: shapes,
      tt: (x) => `match (${x}) { Rect(h: tall, w: wide) => note("rect", [wide, tall]), Circle(r: radius) => note("circle", radius), Point => note("point", null) }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t}.kind === "Rect" ? note("rect", [${t}.w, ${t}.h]) : ${t}.kind === "Circle" ? note("circle", ${t}.r) : note("point", null))`,
    },
    {
      id: "guards",
      title: "guarded arms that fall through to the next arm",
      decls: [shape],
      In: "Shape",
      inputs: "[Shape.Circle(1), Shape.Circle(5), Shape.Rect(4, 2), Shape.Rect(1, 3), Shape.Point]",
      tt: (x) =>
        `match (${x}) { Circle(r) if note("guard big", r > 2) => note("big", r), Circle(r) => note("small", r), Rect(w, h) if note("guard wide", w > h) => note("wide", w), Rect(w, h) => note("tall", h), Point => note("point", 0) }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t}.kind === "Circle" && note("guard big", ${t}.r > 2) ? note("big", ${t}.r) : ${t}.kind === "Circle" ? note("small", ${t}.r) : ${t}.kind === "Rect" && note("guard wide", ${t}.w > ${t}.h) ? note("wide", ${t}.w) : ${t}.kind === "Rect" ? note("tall", ${t}.h) : note("point", 0))`,
    },
    {
      id: "orPattern",
      title: "an or-pattern whose alternatives bind the same field",
      decls: [token],
      In: "Token",
      inputs: tokens,
      tt: (x) => `match (${x}) { Num(value) | Neg(value) => note("number", value), Word(text) => note("word", text), End => note("end", "") }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t}.kind === "Num" || ${t}.kind === "Neg" ? note("number", ${t}.value) : ${t}.kind === "Word" ? note("word", ${t}.text) : note("end", ""))`,
    },
    {
      id: "wildcard",
      title: "a final wildcard arm",
      decls: [token],
      In: "Token",
      inputs: tokens,
      tt: (x) => `match (${x}) { Word(text) => note("word", text.length), _ => note("other", -1) }`,
      ts: (x, [t]) => `(${t} = ${x}, ${t}.kind === "Word" ? note("word", ${t}.text.length) : note("other", -1))`,
    },
    {
      id: "nestedPattern",
      title: "nested patterns over a payload that is a variant",
      decls: [opt, outcome],
      In: "Outcome",
      inputs: "[Outcome.Done(Maybe.Some(7)), Outcome.Done(Maybe.None), Outcome.Failed(\"lost\")]",
      tt: (x) =>
        `match (${x}) { Done(value: Some(value: v)) => note("some", v), Done(value: None()) => note("none", 0), Failed(error) => note("failed", error) }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t}.kind === "Done" && ${t}.value.kind === "Some" ? note("some", ${t}.value.value) : ${t}.kind === "Done" && ${t}.value.kind === "None" ? note("none", 0) : note("failed", ${t}.error))`,
    },
    {
      id: "stringLiterals",
      title: "string literal patterns with an or-pattern",
      In: "string",
      inputs: '["north", "south", "east", "up"]',
      tt: (x) => `match (${x}) { "north" => note("n", 1), "south" | "east" => note("se", 2), _ => note("other", 0) }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t} === "north" ? note("n", 1) : ${t} === "south" || ${t} === "east" ? note("se", 2) : note("other", 0))`,
    },
    {
      id: "numberLiterals",
      title: "number literal patterns written in several notations, with a guard",
      In: "number",
      inputs: "[200, 201, 404, 500, 7]",
      tt: (x) =>
        `match (${x}) { 200 | 0xc9 => note("success", true), 404 => note("missing", false), 500 if note("guard", true) => note("server", false), _ => note("other", null) }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t} === 200 || ${t} === 201 ? note("success", true) : ${t} === 404 ? note("missing", false) : ${t} === 500 && note("guard", true) ? note("server", false) : note("other", null))`,
    },
    {
      id: "booleanLiterals",
      title: "boolean literal patterns without a wildcard",
      In: "boolean",
      inputs: "[true, false]",
      tt: (x) => `match (${x}) { true => note("yes", 1), false => note("no", 0) }`,
      ts: (x, [t]) => `(${t} = ${x}, ${t} === true ? note("yes", 1) : note("no", 0))`,
    },
    {
      id: "tuple",
      title: "a tuple match over two scrutinees",
      decls: [shape, speed],
      In: "Shape",
      inputs: shapes,
      tt: (x) =>
        `match (${x}, note<Speed>("second", Speed.Fast)) { (Circle(r), Fast) => note("fast circle", r), (Rect(w), _) => note("rect", w), (_, Slow) => note("slow", 0), (Circle | Point, _) => note("rest", -1) }`,
      ts: (x, [t, u]) =>
        `(${t} = ${x}, ${u} = note<Speed>("second", Speed.Fast), ${t}.kind === "Circle" && ${u}.kind === "Fast" ? note("fast circle", ${t}.r) : ${t}.kind === "Rect" ? note("rect", ${t}.w) : ${u}.kind === "Slow" ? note("slow", 0) : note("rest", -1))`,
      temps: 2,
    },
    {
      id: "isPatterns",
      title: "`is` patterns over error classes",
      In: "unknown",
      inputs: '[new RangeError("range"), new TypeError("type"), new Error("plain"), "text"]',
      tt: (x) =>
        `match (${x}) { is RangeError { message } => note("range", message), is TypeError | is SyntaxError => note("type", 1), is Error { message: detail } => note("error", detail), _ => note("other", null) }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t} instanceof RangeError ? note("range", ${t}.message) : ${t} instanceof TypeError || ${t} instanceof SyntaxError ? note("type", 1) : ${t} instanceof Error ? note("error", ${t}.message) : note("other", null))`,
    },
    {
      id: "blockArm",
      title: "a block arm whose return is the arm's value",
      decls: [shape],
      In: "Shape",
      inputs: shapes,
      tt: (x) =>
        `match (${x}) { Rect(w, h) => { const area = note("area", w * h); if (area > 10) { return note("large", area); } return note("small", area); }, _ => note("other", 0) }`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t}.kind === "Rect" ? (() => { const area = note("area", ${t}.w * ${t}.h); if (area > 10) { return note("large", area); } return note("small", area); })() : note("other", 0))`,
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
      tt: (x) => `match (${x}) { Click(x) => note("click", x), Key(key) => note("key", key) }`,
      ts: (x, [t]) => `(${t} = ${x}, ${t}.kind === "Click" ? note("click", ${t}.x) : note("key", ${t}.key))`,
    },
    {
      id: "nestedMatch",
      title: "a match whose scrutinee and arm are matches",
      decls: [shape],
      In: "Shape",
      inputs: shapes,
      tt: (x) =>
        `match (match (${x}) { Point => note("inner point", 0), Circle(r) => note("inner circle", r), Rect(w) => note("inner rect", w) }) { 0 => note("zero", "none"), _ => match (note("again", ${"Shape.Point"})) { Point => note("point", "some"), _ => note("other", "?") } }`,
      ts: (x, [t, u, v]) =>
        `(${u} = (${t} = ${x}, ${t}.kind === "Point" ? note("inner point", 0) : ${t}.kind === "Circle" ? note("inner circle", ${t}.r) : note("inner rect", ${t}.w)), ${u} === 0 ? note("zero", "none") : (${v} = note("again", Shape.Point), ${v}.kind === "Point" ? note("point", "some") : note("other", "?")))`,
      temps: 3,
    },
  ],
};
