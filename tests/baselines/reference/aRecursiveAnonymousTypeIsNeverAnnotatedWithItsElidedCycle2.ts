//// [aRecursiveAnonymousTypeIsNeverAnnotatedWithItsElidedCycle2.tt] ////

function mk(k: number) { return { k, m() { return this; } }; }
function pick(n: number) {
  const b = match (n) { 1 => ({ k: 1, m() { return this; } }), _ => ({ k: 2, m() { return this; } }) };
  const c = match (n) { 1 => mk(3), _ => mk(4) };
  return [b.m().m().k, c.m().m().k].join(",");
}
console.log(pick(1), pick(2));

export {};


//// [aRecursiveAnonymousTypeIsNeverAnnotatedWithItsElidedCycle2.ts]

function mk(k: number) { return { k, m() { return this; } }; }
function pick(n: number) {
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
  let $tt_v1;
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 1: {
        $tt_v1 = mk(3);
        break;
      }
      default: {
        $tt_v1 = mk(4);
        break;
      }
    }
  }
  const c = $tt_v1;
  return [b.m().m().k, c.m().m().k].join(",");
}
console.log(pick(1), pick(2));

export {};
