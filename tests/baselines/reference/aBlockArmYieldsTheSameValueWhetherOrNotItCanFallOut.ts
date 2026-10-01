//// [aBlockArmYieldsTheSameValueWhetherOrNotItCanFallOut.tt] ////
variant E { A(v: number), B }
const run = (e: E): unknown => match (e) {
A(v) => {
if (v > 0) { return "positive"; }
throw new Error("not positive");
},
B => "b",
};
const maybe = (e: E): unknown => match (e) {
A(v) => {
if (v > 0) { return "positive"; }
},
B => "b",
};
const never = (e: E): unknown => match (e) {
A(v) => {
void v;
},
B => "b",
};
let threw = "no";
try { run(E.A(-1)); } catch { threw = "yes"; }
console.log([
run(E.A(1)),
threw,
run(E.B),
maybe(E.A(1)),
String(maybe(E.A(-1))),
maybe(E.B),
String(never(E.A(1))),
never(E.B),
].join("|"));

export {};


//// [aBlockArmYieldsTheSameValueWhetherOrNotItCanFallOut.ts]
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
type E =
  | { kind: "A"; v: number }
  | { kind: "B" };
const E = {
  A: (v: number): E => ({ kind: "A", v }),
  B: { kind: "B" } as const,
};
const run = (e: E): unknown => {
  let $tt_v0: unknown;
  {
    const $tt_m = e;
    switch ($tt_m.kind) {
      case "A": {
        const { v } = $tt_m;
        if (v > 0) { $tt_v0 = "positive"; break; }
throw new Error("not positive");
      }
      case "B": {
        $tt_v0 = "b";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
};
const maybe = (e: E): unknown => {
  let $tt_v1: unknown;
  {
    const $tt_m = e;
    switch ($tt_m.kind) {
      case "A": {
        const { v } = $tt_m;
        if (v > 0) { $tt_v1 = "positive"; break; }
          $tt_v1 = undefined;
          break;
      }
      case "B": {
        $tt_v1 = "b";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v1;
};
const never = (e: E): unknown => {
  let $tt_v2: unknown;
  {
    const $tt_m = e;
    switch ($tt_m.kind) {
      case "A": {
        const { v } = $tt_m;
        void v;
          $tt_v2 = undefined;
          break;
      }
      case "B": {
        $tt_v2 = "b";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v2;
};
let threw = "no";
try { run(E.A(-1)); } catch { threw = "yes"; }
console.log([
run(E.A(1)),
threw,
run(E.B),
maybe(E.A(1)),
String(maybe(E.A(-1))),
maybe(E.B),
String(never(E.A(1))),
never(E.B),
].join("|"));

export {};
