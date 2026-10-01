//// [aContextualThisTypeStillReachesTheArmValues1.tt] ////

declare const n: number;
type Methods = { k: number; m(): number } & ThisType<{ q: string }>;
const d: Methods = match (n) { 1 => ({ k: 1, m() { return this.q.length; } }), _ => ({ k: 2, m() { return this.q.length; } }) };
const e: Methods = match (n) { 1 => ({ k: 1, m() { return this.k; } }), _ => ({ k: 2, m() { return 0; } }) };

export {};


//// [aContextualThisTypeStillReachesTheArmValues1.ts]

declare const n: number;
type Methods = { k: number; m(): number } & ThisType<{ q: string }>;
let $tt_v0: Methods;
{
  const $tt_m = n;
  switch ($tt_m) {
    case 1: {
      $tt_v0 = ({ k: 1, m() { return this.q.length; } });
      break;
    }
    default: {
      $tt_v0 = ({ k: 2, m() { return this.q.length; } });
      break;
    }
  }
}
const d: Methods = $tt_v0;
let $tt_v1: Methods;
{
  const $tt_m = n;
  switch ($tt_m) {
    case 1: {
      $tt_v1 = ({ k: 1, m() { return this.k; } });
      break;
    }
    default: {
      $tt_v1 = ({ k: 2, m() { return 0; } });
      break;
    }
  }
}
const e: Methods = $tt_v1;

export {};
