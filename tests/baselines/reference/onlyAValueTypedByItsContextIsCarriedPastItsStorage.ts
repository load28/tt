//// [onlyAValueTypedByItsContextIsCarriedPastItsStorage.tt] ////
declare const n: number;
declare function g(): number;
export const a = match (n) { 1 => 1, _ => g() };
export const b = match (n) { 1 => ({ m() { return this; } }), _ => null };


//// [onlyAValueTypedByItsContextIsCarriedPastItsStorage.ts]
declare const n: number;
declare function g(): number;
let $tt_v0: number;
{
  const $tt_m = n;
  switch ($tt_m) {
    case 1: {
      $tt_v0 = 1;
      break;
    }
    default: {
      $tt_v0 = g();
      break;
    }
  }
}
export const a = $tt_v0;
let $tt_v1;
{
  const $tt_m = n;
  switch ($tt_m) {
    case 1: {
      const $tt_a0 = { value: ({ m() { return this; } }) };
      $tt_v1 = $tt_a0.value;
      break;
    }
    default: {
      $tt_v1 = null;
      break;
    }
  }
}
export const b = $tt_v1;
