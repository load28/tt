//// [eagerArgumentsKeepLeftToRightOrderAtRuntime.tt] ////

const trace: number[] = [];
function mark(n: number): number { trace.push(n); return n; }
function g(a: number, b: number, c: number): void { console.log(a, b, c); }
g(mark(1), match (mark(2)) { 2 => 20, _ => 0 }, mark(3));
console.log(JSON.stringify(trace));

export {};


//// [eagerArgumentsKeepLeftToRightOrderAtRuntime.ts]

const trace: number[] = [];
function mark(n: number): number { trace.push(n); return n; }
function g(a: number, b: number, c: number): void { console.log(a, b, c); }
let $tt_v0: number;
const $tt_v1: typeof g = (g);
const $tt_v2: number = (mark(1));
{
  const $tt_m = mark(2);
  switch ($tt_m) {
    case 2: $tt_v0 = 0; break;
    default: $tt_v0 = 1; break;
  }
}
$tt_v1($tt_v2, ($tt_v0 === 0 ? 20 : 0), mark(3));
console.log(JSON.stringify(trace));

export {};
