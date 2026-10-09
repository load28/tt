//// [aSpreadArgumentBesideATtValueReadsItsElementsFirst.tt] ////
variant V { A, B }
const arr: number[] = [1, 2];
function get(): V { arr.push(7); return V.A; }
const f = (...xs: number[]) => xs.join(",");
class C { xs: number[]; constructor(...xs: number[]) { this.xs = xs; } }
const o: { m?: (...xs: number[]) => string } = { m: f };
const t: [number, string] = [1, "a"];
const g = (n: number, s: string, k: number) => `${n}${s}${k}`;
console.log(f(...arr, match (get()) { A => 1, B => 0 }));
console.log(new C(...arr, match (get()) { A => 1, B => 0 }).xs.join(","));
console.log(o.m?.(...arr, match (get()) { A => 1, B => 0 }));
console.log(g(...t, match (get()) { A => 1, B => 0 }));
console.log(`${arr}|${t}|${match (get()) { A => "a", B => "b" }}`);


//// [aSpreadArgumentBesideATtValueReadsItsElementsFirst.ts]
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
type V =
  | { kind: "A" }
  | { kind: "B" };
const V = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const arr: number[] = [1, 2];
function get(): V { arr.push(7); return V.A; }
const f = (...xs: number[]) => xs.join(",");
class C { xs: number[]; constructor(...xs: number[]) { this.xs = xs; } }
const o: { m?: (...xs: number[]) => string } = { m: f };
const t: [number, string] = [1, "a"];
const g = (n: number, s: string, k: number) => `${n}${s}${k}`;
{
  let $tt_v0: number;
  const $tt_v1: typeof f = (f);
  const $tt_v2 = ($tt_spread(arr));
  {
    const $tt_m = get();
    switch ($tt_m.kind) {
      case "A": $tt_v0 = 0; break;
      case "B": $tt_v0 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  console.log($tt_v1(...$tt_v2, ($tt_v0 === 0 ? 1 : 0)));
}
{
  let $tt_v4: number;
  const $tt_v5: typeof C = (C);
  const $tt_v6 = ($tt_spread(arr));
  {
    const $tt_m = get();
    switch ($tt_m.kind) {
      case "A": $tt_v4 = 0; break;
      case "B": $tt_v4 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  console.log(new $tt_v5(...$tt_v6, ($tt_v4 === 0 ? 1 : 0)).xs.join(","));
}
{
  let $tt_v12: (string) | (undefined);
  const $tt_v9 = (o.m);
  if ($tt_v9 != null) {
    const $tt_v10 = ($tt_spread(arr));
    let $tt_v8: number;
    {
      const $tt_m = get();
      switch ($tt_m.kind) {
        case "A": {
          $tt_v8 = 1;
          break;
        }
        case "B": {
          $tt_v8 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v12 = $tt_v9.call(o, ...$tt_v10, $tt_v8);
  } else {
    $tt_v12 = undefined;
  }
  
  console.log($tt_v12);
}
{
  let $tt_v13: number;
  const $tt_v14: typeof g = (g);
  const $tt_v15 = ($tt_spread(t));
  {
    const $tt_m = get();
    switch ($tt_m.kind) {
      case "A": $tt_v13 = 0; break;
      case "B": $tt_v13 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  console.log($tt_v14(...$tt_v15, ($tt_v13 === 0 ? 1 : 0)));
}
{
  let $tt_v17: number;
  const $tt_v18 = (`${arr}`);
  const $tt_v19 = (`${t}`);
  {
    const $tt_m = get();
    switch ($tt_m.kind) {
      case "A": $tt_v17 = 0; break;
      case "B": $tt_v17 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  console.log(`${$tt_v18}|${$tt_v19}|${($tt_v17 === 0 ? "a" : "b")}`);
}
