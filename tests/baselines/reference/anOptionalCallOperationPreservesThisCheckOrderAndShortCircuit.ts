//// [anOptionalCallOperationPreservesThisCheckOrderAndShortCircuit.tt] ////

const trace: string[] = [];
const live = {
  base: 7,
  m(v: number): number { trace.push("call:" + (this === live)); return this.base + v; },
};
const dead: { m?: (v: number) => number } = {};
function arg(tag: string): number { trace.push(tag); return 1; }
const hit = live.m?.(match (arg("live")) { 1 => 1, _ => 0 });
const miss = dead.m?.(match (arg("dead")) { 1 => 1, _ => 0 });
console.log(JSON.stringify(trace), hit, miss);

export {};


//// [anOptionalCallOperationPreservesThisCheckOrderAndShortCircuit.ts]

const trace: string[] = [];
const live = {
  base: 7,
  m(v: number): number { trace.push("call:" + (this === live)); return this.base + v; },
};
const dead: { m?: (v: number) => number } = {};
function arg(tag: string): number { trace.push(tag); return 1; }
let $tt_v2: (number) | (undefined);
const $tt_v1 = (live.m);
if ($tt_v1 != null) {
  {
    const $tt_m = arg("live");
    switch ($tt_m) {
      case 1: {
        $tt_v2 = $tt_v1.call(live, 1);
        break;
      }
      default: {
        $tt_v2 = $tt_v1.call(live, 0);
        break;
      }
    }
  }
} else {
  $tt_v2 = undefined;
}

const hit = $tt_v2;
let $tt_v5: (number) | (undefined);
const $tt_v4 = (dead.m);
if ($tt_v4 != null) {
  {
    const $tt_m = arg("dead");
    switch ($tt_m) {
      case 1: {
        $tt_v5 = $tt_v4.call(dead, 1);
        break;
      }
      default: {
        $tt_v5 = $tt_v4.call(dead, 0);
        break;
      }
    }
  }
} else {
  $tt_v5 = undefined;
}

const miss = $tt_v5;
console.log(JSON.stringify(trace), hit, miss);

export {};
