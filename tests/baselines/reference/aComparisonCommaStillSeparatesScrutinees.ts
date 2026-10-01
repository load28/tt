//// [aComparisonCommaStillSeparatesScrutinees.tt] ////
declare const a: number, b: number, c: number;
export const r = match (a < b, c > a) { (_, _) => 1 };


//// [aComparisonCommaStillSeparatesScrutinees.ts]
declare const a: number, b: number, c: number;
let $tt_v0: number;
{
  const $tt_m0 = a < b;
  const $tt_m1 = c > a;
  do {
    $tt_v0 = 1;
    break;
  } while (false);
}
export const r = $tt_v0;
