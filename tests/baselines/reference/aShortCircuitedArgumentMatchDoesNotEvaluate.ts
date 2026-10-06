//// [aShortCircuitedArgumentMatchDoesNotEvaluate.tt] ////

const trace: string[] = [];
function subject(tag: string): number { trace.push(tag); return 1; }
function id(v: number): number { return v; }
declare const globalThis: { flagOn: boolean };
const on = true as boolean;
const off = false as boolean;
const a = on && id(match (subject("on")) { 1 => 10, _ => 0 });
const b = off && id(match (subject("off")) { 1 => 20, _ => 0 });
console.log(JSON.stringify(trace), a, b);

export {};


//// [aShortCircuitedArgumentMatchDoesNotEvaluate.ts]

const trace: string[] = [];
function subject(tag: string): number { trace.push(tag); return 1; }
function id(v: number): number { return v; }
declare const globalThis: { flagOn: boolean };
const on = true as boolean;
const off = false as boolean;
let $tt_v3: (number) | (false);
let $tt_v2: boolean;
if ($tt_v2 = on) {
  let $tt_v0: number;
  const $tt_v1: typeof id = (id);
  {
    const $tt_m = subject("on");
    switch ($tt_m) {
      case 1: {
        $tt_v0 = 10;
        break;
      }
      default: {
        $tt_v0 = 0;
        break;
      }
    }
  }
  $tt_v3 = $tt_v2 && $tt_v1($tt_v0);
} else {
  $tt_v3 = $tt_v2;
}

const a = $tt_v3;
let $tt_v7: (number) | (false);
let $tt_v6: boolean;
if ($tt_v6 = off) {
  let $tt_v4: number;
  const $tt_v5: typeof id = (id);
  {
    const $tt_m = subject("off");
    switch ($tt_m) {
      case 1: {
        $tt_v4 = 20;
        break;
      }
      default: {
        $tt_v4 = 0;
        break;
      }
    }
  }
  $tt_v7 = $tt_v6 && $tt_v5($tt_v4);
} else {
  $tt_v7 = $tt_v6;
}

const b = $tt_v7;
console.log(JSON.stringify(trace), a, b);

export {};
