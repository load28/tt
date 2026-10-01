//// [siblingValuesBesideAShortCircuitKeepLeftToRightOrder.tt] ////

const trace: number[] = [];
function mark(n: number): number { trace.push(n); return n; }
function g(x: unknown, y: unknown): void { console.log(x, y); }
const a = true as boolean;
g(a && match (mark(1)) { 1 => 11, _ => 0 }, match (mark(2)) { 2 => 22, _ => 0 });
console.log(JSON.stringify(trace));

export {};


//// [siblingValuesBesideAShortCircuitKeepLeftToRightOrder.ts]

const trace: number[] = [];
function mark(n: number): number { trace.push(n); return n; }
function g(x: unknown, y: unknown): void { console.log(x, y); }
const a = true as boolean;
let $tt_subject;
let $tt_subject_1;

g(a && ($tt_subject = mark(1), ($tt_subject === 1) ? 11 : 0), ($tt_subject_1 = mark(2), ($tt_subject_1 === 2) ? 22 : 0));
console.log(JSON.stringify(trace));

export {};
