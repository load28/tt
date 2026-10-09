//// [aSpreadBesideATtValueReadsItsElementsFirst.tt] ////
variant V { A, B }
const o: { a: number } = { a: 1 };
const arr = [1, 2];
function get(): V { o.a = 99; arr.push(7); return V.A; }
function bump(v: V): number { o.a = 98; return v.kind === "A" ? 1 : 2; }
export function objectScrutinee() {
  return { ...o, b: match (get()) { A => 1, B => 0 } };
}
export function arrayScrutinee() {
  return [...arr, match (get()) { A => 1, B => 0 }];
}
export function objectArm(v: V) {
  o.a = 1;
  return { ...o, b: match (v) { A => bump(v), B => 0 } };
}
export function arrayArm(v: V) {
  arr.length = 2;
  return [...arr, match (v) { A => arr.push(3), B => 0 }];
}
console.log(JSON.stringify(objectScrutinee()));
console.log(JSON.stringify(arrayScrutinee()));
console.log(JSON.stringify(objectArm(V.A)));
console.log(JSON.stringify(arrayArm(V.A)));


//// [aSpreadBesideATtValueReadsItsElementsFirst.ts]
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
function $tt_spread<T extends readonly unknown[]>(values: T): [...T];
function $tt_spread<T>(values: Iterable<T>): T[];
function $tt_spread(values: Iterable<unknown>) {
  return [...values];
}
type V =
  | { kind: "A" }
  | { kind: "B" };
const V = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const o: { a: number } = { a: 1 };
const arr = [1, 2];
function get(): V { o.a = 99; arr.push(7); return V.A; }
function bump(v: V): number { o.a = 98; return v.kind === "A" ? 1 : 2; }
export function objectScrutinee() {
  let $tt_v0: number;
  const $tt_v1 = ({ ...o });
  {
    const $tt_m = get();
    switch ($tt_m.kind) {
      case "A": $tt_v0 = 0; break;
      case "B": $tt_v0 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  return { ...$tt_v1, b: ($tt_v0 === 0 ? 1 : 0) };
}
export function arrayScrutinee() {
  let $tt_v2: number;
  const $tt_v3 = ($tt_spread(arr));
  {
    const $tt_m = get();
    switch ($tt_m.kind) {
      case "A": $tt_v2 = 0; break;
      case "B": $tt_v2 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  return [...$tt_v3, ($tt_v2 === 0 ? 1 : 0)];
}
export function objectArm(v: V) {
  o.a = 1;
  let $tt_v4: number;
  const $tt_v5 = ({ ...o });
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": $tt_v4 = 0; break;
      case "B": $tt_v4 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  return { ...$tt_v5, b: ($tt_v4 === 0 ? bump(v) : 0) };
}
export function arrayArm(v: V) {
  arr.length = 2;
  let $tt_v6: number;
  const $tt_v7 = ($tt_spread(arr));
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": $tt_v6 = 0; break;
      case "B": $tt_v6 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  return [...$tt_v7, ($tt_v6 === 0 ? arr.push(3) : 0)];
}
console.log(JSON.stringify(objectScrutinee()));
console.log(JSON.stringify(arrayScrutinee()));
console.log(JSON.stringify(objectArm(V.A)));
console.log(JSON.stringify(arrayArm(V.A)));
