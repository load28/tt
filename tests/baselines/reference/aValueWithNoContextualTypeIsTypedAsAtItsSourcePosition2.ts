//// [aValueWithNoContextualTypeIsTypedAsAtItsSourcePosition2.tt] ////

function pick(n: number) {
  const b = match (n) { 1 => ({ k: 1, m() { return this; } }), _ => ({ k: 2, m() { return this; } }) };
  const f = match (n) { 1 => () => b.m().k, _ => function () { return 0; } };
  const xs = match (n) { 1 => [n], _ => [] };
  return [b.m().k, f(), xs.length].join(",");
}
console.log(pick(1), pick(2));

export {};


//// [aValueWithNoContextualTypeIsTypedAsAtItsSourcePosition2.ts]

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
  let $tt_v1: () => number;
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 1: {
        const $tt_a2 = { value: () => b.m().k };
        $tt_v1 = $tt_a2.value;
        break;
      }
      default: {
        const $tt_a3 = { value: function () { return 0; } };
        $tt_v1 = $tt_a3.value;
        break;
      }
    }
  }
  const f = $tt_v1;
  let $tt_v2: number[];
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 1: {
        const $tt_a4 = { value: [n] };
        $tt_v2 = $tt_a4.value;
        break;
      }
      default: {
        const $tt_a5 = { value: [] };
        $tt_v2 = $tt_a5.value;
        break;
      }
    }
  }
  const xs = $tt_v2;
  return [b.m().k, f(), xs.length].join(",");
}
console.log(pick(1), pick(2));

export {};
