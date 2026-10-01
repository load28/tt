//// [capturedTtExpressionsAndStatementBodiesEvaluateOnceInOrder.tt] ////

variant E { A(value: number), B }
const events: string[] = [];
function note(n: number) { events.push(`n:${n}`); return n; }
function callable() { events.push("callee"); return (n: number) => { events.push(`call:${n}`); return n; }; }
const values = [(note(1) |> ((n: number) => note(n + 1))), match(note(3)) { 3 => 3, _ => 0 }];
const calls = callable()(match(note(4)){4=>4,_=>0}) + callable()(match(note(5)){5=>5,_=>0});
const statements = [(() => { if let A(value) = E.A(note(6)) { return value; } return 0; })(), match(note(7)){7=>7,_=>0}];
const bindings = [(() => { const A(value) = E.A(note(8)) else { return 0; }; return value; })(), match(note(9)){9=>9,_=>0}];
console.log(JSON.stringify([values, calls, statements, bindings]));
console.log(events.join(","));

export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [capturedTtExpressionsAndStatementBodiesEvaluateOnceInOrder.ts]
import { $tt_ap } from "./tt/runtime.js";

type E =
  | { kind: "A"; value: number }
  | { kind: "B" };
const E = {
  A: (value: number): E => ({ kind: "A", value }),
  B: { kind: "B" } as const,
};
const events: string[] = [];
function note(n: number) { events.push(`n:${n}`); return n; }
function callable() { events.push("callee"); return (n: number) => { events.push(`call:${n}`); return n; }; }
let $tt_v1: number;
const $tt_v2 = ($tt_ap(note(1), ((n: number) => note(n + 1))));
{
  const $tt_m = note(3);
  switch ($tt_m) {
    case 3: {
      $tt_v1 = 3;
      break;
    }
    default: {
      $tt_v1 = 0;
      break;
    }
  }
}
const values = [($tt_v2), $tt_v1];
let $tt_subject_1;
let $tt_subject_2;

const calls = callable()(($tt_subject_1 = note(4), ($tt_subject_1 === 4) ? 4 : 0)) + callable()(($tt_subject_2 = note(5), ($tt_subject_2 === 5) ? 5 : 0));
let $tt_v8: number;
const $tt_v9 = ((() => { {
  const $tt_t0 = E.A(note(6));
  if ($tt_t0.kind === "A") {
    const { value } = $tt_t0;
    return value;
  }
} return 0; })());
{
  const $tt_m = note(7);
  switch ($tt_m) {
    case 7: $tt_v8 = 0; break;
    default: $tt_v8 = 1; break;
  }
}
const statements = [$tt_v9, ($tt_v8 === 0 ? 7 : 0)];
let $tt_v10: number;
const $tt_v11 = ((() => { const $tt_t1 = E.A(note(8));
if ($tt_t1.kind !== "A") {
  return 0;
}
const { value } = $tt_t1; return value; })());
{
  const $tt_m = note(9);
  switch ($tt_m) {
    case 9: $tt_v10 = 0; break;
    default: $tt_v10 = 1; break;
  }
}
const bindings = [$tt_v11, ($tt_v10 === 0 ? 9 : 0)];
console.log(JSON.stringify([values, calls, statements, bindings]));
console.log(events.join(","));

export {};
