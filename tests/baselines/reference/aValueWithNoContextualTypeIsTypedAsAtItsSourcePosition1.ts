//// [aValueWithNoContextualTypeIsTypedAsAtItsSourcePosition1.tt] ////

declare const n: number;
const b = match (n) { 1 => ({ k: 1, m() { return this; } }), _ => ({ k: 2, m() { return this; } }) };
b.m().zzz;
const xs = match (n) { 1 => [n], _ => [] };
const s = match (n) { 1 => Symbol(), _ => Symbol() };
const checked: symbol[] = [s];
xs.push(1);

export {};


//// [aValueWithNoContextualTypeIsTypedAsAtItsSourcePosition1.ts]

declare const n: number;
let $tt_v0;
{
  const $tt_m = n;
  switch ($tt_m) {
    case 1: {
      const $tt_a0 = { value: ({ k: 1, m() { return this; } }) };
      $tt_v0 = $tt_a0.value;
      break;
    }
    default: {
      const $tt_a1 = { value: ({ k: 2, m() { return this; } }) };
      $tt_v0 = $tt_a1.value;
      break;
    }
  }
}
const b = $tt_v0;
b.m().zzz;
let $tt_v1: number[];
{
  const $tt_m = n;
  switch ($tt_m) {
    case 1: {
      const $tt_a2 = { value: [n] };
      $tt_v1 = $tt_a2.value;
      break;
    }
    default: {
      const $tt_a3 = { value: [] };
      $tt_v1 = $tt_a3.value;
      break;
    }
  }
}
const xs = $tt_v1;
let $tt_v2: symbol;
{
  const $tt_m = n;
  switch ($tt_m) {
    case 1: {
      $tt_v2 = Symbol();
      break;
    }
    default: {
      $tt_v2 = Symbol();
      break;
    }
  }
}
const s = $tt_v2;
const checked: symbol[] = [s];
xs.push(1);

export {};
