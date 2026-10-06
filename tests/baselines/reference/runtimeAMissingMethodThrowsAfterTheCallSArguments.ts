//// [runtimeAMissingMethodThrowsAfterTheCallSArguments.tt] ////

const trace: string[] = [];
function m(): number { trace.push("m"); return 1; }
type O = { k: number; add(x: number): number; id<T>(v: T): T; missing?: (x: number) => number };
const o: O = { k: 5, add(x: number) { return x + this.k; }, id<T>(v: T): T { return v; } };
function attempt(f: () => unknown): string {
  try { return String(f()); } catch (e) { return (e as Error).constructor.name; }
}
const missing = attempt(() => o.missing!(match (m()) { 1 => 10, _ => 20 }));
const n: number = o.id(match (m()) { 1 => 10, _ => 20 });
const piped = o |> .add(match (m()) { 1 => 1, _ => 2 });
console.log(missing, n, o.add(match (m()) { 1 => 10, _ => 20 }), piped, JSON.stringify(trace));

export {};


//// [runtimeAMissingMethodThrowsAfterTheCallSArguments.ts]

const trace: string[] = [];
function m(): number { trace.push("m"); return 1; }
type O = { k: number; add(x: number): number; id<T>(v: T): T; missing?: (x: number) => number };
const o: O = { k: 5, add(x: number) { return x + this.k; }, id<T>(v: T): T { return v; } };
function attempt(f: () => unknown): string {
  try { return String(f()); } catch (e) { return (e as Error).constructor.name; }
}
const missing = attempt(() => {
  let $tt_v0: number;
  {
    const $tt_m = m();
    switch ($tt_m) {
      case 1: $tt_v0 = 0; break;
      default: $tt_v0 = 1; break;
    }
  }
  return o.missing!(($tt_v0 === 0 ? 10 : 20));
});
let $tt_v2: number;
{
  const $tt_m = m();
  switch ($tt_m) {
    case 1: $tt_v2 = 0; break;
    default: $tt_v2 = 1; break;
  }
}
const n: number = o.id(($tt_v2 === 0 ? 10 : 20));
let $tt_v4: number;
do {
  const $tt_v13 = o;
  let $tt_v5: number;
  const $tt_v7 = ($tt_v13);
  {
    const $tt_m = m();
    switch ($tt_m) {
      case 1: {
        $tt_v5 = 1;
        break;
      }
      default: {
        $tt_v5 = 2;
        break;
      }
    }
  }
  $tt_v4 = $tt_v7.add($tt_v5);
  break;
} while (false);
const piped = $tt_v4;
let $tt_v8: number;
const $tt_v11: typeof missing = (missing);
const $tt_v12: typeof n = (n);
{
  const $tt_m = m();
  switch ($tt_m) {
    case 1: $tt_v8 = 0; break;
    default: $tt_v8 = 1; break;
  }
}
console.log($tt_v11, $tt_v12, o.add(($tt_v8 === 0 ? 10 : 20)), piped, JSON.stringify(trace));

export {};
