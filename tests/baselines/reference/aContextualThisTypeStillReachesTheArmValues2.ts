//// [aContextualThisTypeStillReachesTheArmValues2.tt] ////

type Methods = { k: number; m(): number } & ThisType<{ q: string }>;
function pick(n: number) {
  const d: Methods = match (n) { 1 => ({ k: 1, m() { return this.q.length; } }), _ => ({ k: 2, m() { return 0; } }) };
  return d.m.call({ q: "abc" });
}
console.log(pick(1), pick(2));

export {};


//// [aContextualThisTypeStillReachesTheArmValues2.ts]

type Methods = { k: number; m(): number } & ThisType<{ q: string }>;
function pick(n: number) {
  let $tt_v0: Methods;
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 1: {
        $tt_v0 = ({ k: 1, m() { return this.q.length; } });
        break;
      }
      default: {
        $tt_v0 = ({ k: 2, m() { return 0; } });
        break;
      }
    }
  }
  const d: Methods = $tt_v0;
  return d.m.call({ q: "abc" });
}
console.log(pick(1), pick(2));

export {};
