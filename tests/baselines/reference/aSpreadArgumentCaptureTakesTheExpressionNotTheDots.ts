//// [aSpreadArgumentCaptureTakesTheExpressionNotTheDots.tt] ////
declare function sum(...xs: number[]): number;
declare const rest: number[];
export const r = sum(...rest, match (1) { 1 => 3, _ => 0 });


//// [aSpreadArgumentCaptureTakesTheExpressionNotTheDots.ts]
declare function sum(...xs: number[]): number;
declare const rest: number[];
let $tt_v0: number;
const $tt_v1 = (sum);
const $tt_v2 = (rest);
{
  const $tt_m = 1;
  switch ($tt_m) {
    case 1: $tt_v0 = 0; break;
    default: $tt_v0 = 1; break;
  }
}
export const r = $tt_v1(...$tt_v2, ($tt_v0 === 0 ? 3 : 0));
