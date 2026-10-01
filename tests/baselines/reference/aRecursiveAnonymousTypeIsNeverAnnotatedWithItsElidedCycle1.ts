//// [aRecursiveAnonymousTypeIsNeverAnnotatedWithItsElidedCycle1.tt] ////

declare const n: number;
const b = match (n) { 1 => ({ k: 1, m() { return this; } }), _ => ({ k: 2, m() { return this; } }) };
b.m().m().zzz;
function mk() { return { m() { return this; } }; }
const c = match (n) { 1 => mk(), _ => mk() };
c.m().m().yyy;
const d = match (n) { 1 => [JSON.parse("1")], _ => [] };
d[0].anything;

export {};


//// [aRecursiveAnonymousTypeIsNeverAnnotatedWithItsElidedCycle1.ts]

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
b.m().m().zzz;
function mk() { return { m() { return this; } }; }
let $tt_v1;
{
  const $tt_m = n;
  switch ($tt_m) {
    case 1: {
      $tt_v1 = mk();
      break;
    }
    default: {
      $tt_v1 = mk();
      break;
    }
  }
}
const c = $tt_v1;
c.m().m().yyy;
let $tt_v2: any[];
{
  const $tt_m = n;
  switch ($tt_m) {
    case 1: {
      const $tt_a2 = { value: [JSON.parse("1")] };
      $tt_v2 = $tt_a2.value;
      break;
    }
    default: {
      const $tt_a3 = { value: [] };
      $tt_v2 = $tt_a3.value;
      break;
    }
  }
}
const d = $tt_v2;
d[0].anything;

export {};
