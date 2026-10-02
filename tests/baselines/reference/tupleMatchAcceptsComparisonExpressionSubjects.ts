//// [tupleMatchAcceptsComparisonExpressionSubjects.tt] ////
variant V { A, B }
declare const a: number; declare const b: number; declare const v: V;
declare const id: <T>(value: T) => T;
const n = match (a < b, v) { (_, A) => 1, _ => 0 };
const m = match (id<number>(0), v) { (_, A) => 1, _ => 0 };
const k = match ((a < b), (b > (a)), v) { (_, _, A) => 1, _ => 0 };
const j = match (a < b, (b > (a)), v) { (_, _, A) => 1, _ => 0 };


//// [tupleMatchAcceptsComparisonExpressionSubjects.ts]
type V =
  | { kind: "A" }
  | { kind: "B" };
const V = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
declare const a: number; declare const b: number; declare const v: V;
declare const id: <T>(value: T) => T;
let $tt_v0$n: number;
{
  const $tt_m0 = a < b;
  const $tt_m1 = v;
  do {
    if ($tt_m1.kind === "A") {
      $tt_v0$n = 1;
      break;
    }
    $tt_v0$n = 0;
    break;
  } while (false);
}
const n = $tt_v0$n;
let $tt_v1$m: number;
{
  const $tt_m0 = id<number>(0);
  const $tt_m1 = v;
  do {
    if ($tt_m1.kind === "A") {
      $tt_v1$m = 1;
      break;
    }
    $tt_v1$m = 0;
    break;
  } while (false);
}
const m = $tt_v1$m;
let $tt_v2$k: number;
{
  const $tt_m0 = (a < b);
  const $tt_m1 = (b > (a));
  const $tt_m2 = v;
  do {
    if ($tt_m2.kind === "A") {
      $tt_v2$k = 1;
      break;
    }
    $tt_v2$k = 0;
    break;
  } while (false);
}
const k = $tt_v2$k;
let $tt_v3$j: number;
{
  const $tt_m0 = a < b;
  const $tt_m1 = (b > (a));
  const $tt_m2 = v;
  do {
    if ($tt_m2.kind === "A") {
      $tt_v3$j = 1;
      break;
    }
    $tt_v3$j = 0;
    break;
  } while (false);
}
const j = $tt_v3$j;
