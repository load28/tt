//// [aCStyleLoopTestCapturesItsLeftOperandOnce.tt] ////

const xs = [3];
let j = 0;
const seen: number[] = [];
for (; j < match (xs[0]) { 3 => 3, _ => 0 };) { seen.push(j); j++; }
for (let k = 0; k < match (xs[0]) { 3 => 2, _ => 0 }; k++) { seen.push(10 + k); }
console.log(JSON.stringify(seen));

export {};


//// [aCStyleLoopTestCapturesItsLeftOperandOnce.ts]

const xs = [3];
let j = 0;
const seen: number[] = [];
for (; ; ) {
  let $tt_v0: number;
  const $tt_v1: typeof j = (j);
  {
    const $tt_m = xs[0];
    switch ($tt_m) {
      case 3: {
        $tt_v0 = 3;
        break;
      }
      default: {
        $tt_v0 = 0;
        break;
      }
    }
  }
  if (!($tt_v1 < $tt_v0)) break; { seen.push(j); j++; }}
for (let k = 0; ; k++) {
  let $tt_v2: number;
  const $tt_v3: typeof k = (k);
  {
    const $tt_m = xs[0];
    switch ($tt_m) {
      case 3: {
        $tt_v2 = 2;
        break;
      }
      default: {
        $tt_v2 = 0;
        break;
      }
    }
  }
  if (!($tt_v3 < $tt_v2)) break; { seen.push(10 + k); }}
console.log(JSON.stringify(seen));

export {};
