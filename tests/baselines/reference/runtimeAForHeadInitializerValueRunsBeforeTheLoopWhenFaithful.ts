//// [runtimeAForHeadInitializerValueRunsBeforeTheLoopWhenFaithful.tt] ////

variant O { A(n: number), B }
const o = O.A(1) as O;
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function r(n: number): R { return n > 0 ? { kind: "Ok", value: n } : { kind: "Err", error: "e" + n }; }
const out: string[] = [];
function a() {
  for (let f = match (o) { A(n) => () => n + 1, B => null }, i = 0; i < 1; i++) out.push("a" + f!() + i);
}
function c(n: number): R {
  for (let x = try r(n), i = 0; i < 2; i++) out.push("c" + x + i);
  return { kind: "Ok", value: 1 };
}
function d() {
  for (let n = match (o) { A(n) => n, B => 0 }; n < 3; n++) out.push("d" + n);
}
function f() {
  for (let k: ((k: number) => number) | null = match (o) { A(n) => (k: number) => k + n, B => null }; k; k = null) out.push("f" + k(1));
}
function g() {
  for (var v: (() => unknown) | null = match (o) { A(n) => () => v, B => null }; v; v = null) out.push("g" + (v() === v));
}
function e() {
  let x = 0;
  for (x = match (o) { A(n) => n + 10, B => 0 }; x < 12; x++) out.push("e" + x);
}
function p(n: number): R {
  let y = 0;
  for (y = try r(n); y < 2; y++) out.push("p" + y);
  return { kind: "Ok", value: y };
}
a();
console.log(JSON.stringify(c(1)), JSON.stringify(c(0)));
d(); f(); g(); e();
console.log(JSON.stringify(p(1)), JSON.stringify(p(-1)));
console.log(out.join(" "));

export {};


//// [runtimeAForHeadInitializerValueRunsBeforeTheLoopWhenFaithful.ts]
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

type O =
  | { kind: "A"; n: number }
  | { kind: "B" };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const o = O.A(1) as O;
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function r(n: number): R { return n > 0 ? { kind: "Ok", value: n } : { kind: "Err", error: "e" + n }; }
const out: string[] = [];
function a() {
  let $tt_v0: (() => number) | (null);
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        const $tt_a0 = { value: () => n + 1 };
        $tt_v0 = $tt_a0.value;
        break;
      }
      case "B": {
        $tt_v0 = null;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  for (let f = $tt_v0, i = 0; i < 1; i++) out.push("a" + f!() + i);
}
function c(n: number): R {
  let $tt_v1: number;
  const $tt_t0 = r(n);
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v1 = $tt_t0.value;
  for (let x = $tt_v1, i = 0; i < 2; i++) out.push("c" + x + i);
  return { kind: "Ok", value: 1 };
}
function d() {
  let $tt_v2: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v2 = n;
        break;
      }
      case "B": {
        $tt_v2 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  for (let n = $tt_v2; n < 3; n++) out.push("d" + n);
}
function f() {
  let $tt_v3: ((k: number) => number) | null;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v3 = (k: number) => k + n;
        break;
      }
      case "B": {
        $tt_v3 = null;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  for (let k: ((k: number) => number) | null = $tt_v3; k; k = null) out.push("f" + k(1));
}
function g() {
  let $tt_v4: (() => unknown) | null;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v4 = () => v;
        break;
      }
      case "B": {
        $tt_v4 = null;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  for (var v: (() => unknown) | null = $tt_v4; v; v = null) out.push("g" + (v() === v));
}
function e() {
  let x = 0;
  let $tt_v5: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v5 = n + 10;
        break;
      }
      case "B": {
        $tt_v5 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  for (x = $tt_v5; x < 12; x++) out.push("e" + x);
}
function p(n: number): R {
  let y = 0;
  let $tt_v6: number;
  const $tt_t1 = r(n);
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  $tt_v6 = $tt_t1.value;
  for (y = $tt_v6; y < 2; y++) out.push("p" + y);
  return { kind: "Ok", value: y };
}
a();
console.log(JSON.stringify(c(1)), JSON.stringify(c(0)));
d(); f(); g(); e();
console.log(JSON.stringify(p(1)), JSON.stringify(p(-1)));
console.log(out.join(" "));

export {};
