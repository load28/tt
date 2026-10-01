//// [aSemicolonFreeBraceAfterAnExpressionIsADivergingBlock.tt] ////
type Opt = { kind: "Some"; value: number } | { kind: "None" }
let foo = 0, Foo = 0
function unwrap(o: Opt): number {
let Some(value) = o else {
foo
Foo
{ return -1 }
};
return value
}
function arm(o: Opt): number {
const n: number = match (o) {
Some(value) => {
foo
{ return value * 2 }
},
None => 0,
}
return n
}
function body(o: Opt): number {
if let Some(value) = o {
Foo
{ return value + 1 }
} else {
foo
{ return -2 }
}
}
console.log([unwrap({ kind: "Some", value: 3 }), unwrap({ kind: "None" })].join(","))
console.log([arm({ kind: "Some", value: 3 }), body({ kind: "Some", value: 3 }), body({ kind: "None" })].join(","))

export {};


//// [aSemicolonFreeBraceAfterAnExpressionIsADivergingBlock.ts]
function $tt_show(value: unknown): string {
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return String(value);
}
type Opt = { kind: "Some"; value: number } | { kind: "None" }
let foo = 0, Foo = 0
function unwrap(o: Opt): number {
const $tt_t0 = o;
if ($tt_t0.kind !== "Some") {
  foo
Foo
{ return -1 }
}
let { value } = $tt_t0;
return value
}
function arm(o: Opt): number {
let $tt_v0: number;
{
  const $tt_m = o;
  switch ($tt_m.kind) {
    case "Some": {
      const { value } = $tt_m;
      foo
{ $tt_v0 = value * 2; break; }
    }
    case "None": {
      $tt_v0 = 0;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const n: number = $tt_v0
return n
}
function body(o: Opt): number {
{
  const $tt_t1 = o;
  if ($tt_t1.kind === "Some") {
    const { value } = $tt_t1;
    Foo
{ return value + 1 }
  } else {
    foo
{ return -2 }
  }
}
}
console.log([unwrap({ kind: "Some", value: 3 }), unwrap({ kind: "None" })].join(","))
console.log([arm({ kind: "Some", value: 3 }), body({ kind: "Some", value: 3 }), body({ kind: "None" })].join(","))

export {};
