//// [anInertArgumentIsNotCapturedButAnEffectfulOneIs.tt] ////
declare function g(a: number, b: number, c: number): void;
declare function eff(): number;
g(1, match (1) { 1 => 1, _ => 0 }, 2);
g(eff(), match (1) { 1 => 1, _ => 0 }, 2);


//// [anInertArgumentIsNotCapturedButAnEffectfulOneIs.ts]
declare function g(a: number, b: number, c: number): void;
declare function eff(): number;
{
  let $tt_v0: number;
  const $tt_v1 = (g);
  {
    const $tt_m = 1;
    switch ($tt_m) {
      case 1: $tt_v0 = 0; break;
      default: $tt_v0 = 1; break;
    }
  }
  $tt_v1(1, ($tt_v0 === 0 ? 1 : 0), 2);
}
{
  let $tt_v2: number;
  const $tt_v3 = (g);
  const $tt_v4: number = (eff());
  {
    const $tt_m = 1;
    switch ($tt_m) {
      case 1: $tt_v2 = 0; break;
      default: $tt_v2 = 1; break;
    }
  }
  $tt_v3($tt_v4, ($tt_v2 === 0 ? 1 : 0), 2);
}
