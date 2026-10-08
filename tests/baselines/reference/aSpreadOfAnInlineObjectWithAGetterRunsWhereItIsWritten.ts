//// [aSpreadOfAnInlineObjectWithAGetterRunsWhereItIsWritten.tt] ////
variant S { A, B }
const log: string[] = [];
function o(): S { log.push("subject"); return S.A; }
const p = new Proxy({ a: 1 }, { ownKeys(t) { log.push("ownKeys"); return Reflect.ownKeys(t); } });
const it = { *[Symbol.iterator]() { log.push("iter"); yield 1; } };
function main() {
  const a = { ...{ ...p }, x: match (o()) { A => 1, B => 2 } };
  log.push("|");
  const b = { ...{ get g() { log.push("getter"); return 1; } }, x: match (o()) { A => 1, B => 2 } };
  log.push("|");
  const c = [...[...it], match (o()) { A => 1, B => 2 }];
  log.push("|");
  const d = { ...[...it], x: match (o()) { A => 1, B => 2 } };
  log.push("|");
  const e: { g: number; x: number } = { ...{ get g() { log.push("typed getter"); return 1; } }, x: match (o()) { A => 1, B => 2 } };
  return [a, b, c, d, e];
}
main();
console.log(log.join(", "));


//// [aSpreadOfAnInlineObjectWithAGetterRunsWhereItIsWritten.ts]
var $tt_show: (value: unknown) => string = function (value) {
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
};
var $tt_spread: {
  <T extends readonly unknown[]>(values: T): [...T];
  <T>(values: Iterable<T>): T[];
} = function (values: Iterable<unknown>) {
  return [...values];
};
type S =
  | { kind: "A" }
  | { kind: "B" };
const S = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const log: string[] = [];
function o(): S { log.push("subject"); return S.A; }
const p = new Proxy({ a: 1 }, { ownKeys(t) { log.push("ownKeys"); return Reflect.ownKeys(t); } });
const it = { *[Symbol.iterator]() { log.push("iter"); yield 1; } };
function main() {
  let $tt_v0: number;
  const $tt_v1 = ({ ...{ ...p } });
  {
    const $tt_m = o();
    switch ($tt_m.kind) {
      case "A": $tt_v0 = 0; break;
      case "B": $tt_v0 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  const a = { ...$tt_v1, x: ($tt_v0 === 0 ? 1 : 2) };
  log.push("|");
  let $tt_v2: number;
  const $tt_v3 = ({ ...{ get g() { log.push("getter"); return 1; } } });
  {
    const $tt_m = o();
    switch ($tt_m.kind) {
      case "A": $tt_v2 = 0; break;
      case "B": $tt_v2 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  const b = { ...$tt_v3, x: ($tt_v2 === 0 ? 1 : 2) };
  log.push("|");
  let $tt_v4: number;
  const $tt_v5 = ($tt_spread([...it]));
  {
    const $tt_m = o();
    switch ($tt_m.kind) {
      case "A": $tt_v4 = 0; break;
      case "B": $tt_v4 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  const c = [...$tt_v5, ($tt_v4 === 0 ? 1 : 2)];
  log.push("|");
  let $tt_v6: number;
  const $tt_v7 = ({ ...[...it] });
  {
    const $tt_m = o();
    switch ($tt_m.kind) {
      case "A": $tt_v6 = 0; break;
      case "B": $tt_v6 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  const d = { ...$tt_v7, x: ($tt_v6 === 0 ? 1 : 2) };
  log.push("|");
  let $tt_v8: number;
  const $tt_v9 = ({ ...{ get g() { log.push("typed getter"); return 1; } } });
  {
    const $tt_m = o();
    switch ($tt_m.kind) {
      case "A": $tt_v8 = 0; break;
      case "B": $tt_v8 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  const e: { g: number; x: number } = { ...$tt_v9, x: ($tt_v8 === 0 ? 1 : 2) };
  return [a, b, c, d, e];
}
main();
console.log(log.join(", "));
