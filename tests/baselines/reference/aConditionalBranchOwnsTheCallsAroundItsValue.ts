//// [aConditionalBranchOwnsTheCallsAroundItsValue.tt] ////

variant O { A, B }
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const Ok = <T,>(value: T): R<T> => ({ kind: "Ok", value });
const Err = (error: string): R<never> => ({ kind: "Err", error });
const log: string[] = [];
const f = (n: number) => { log.push("f" + n); return n * 10; };
const g = (n: number) => { log.push("g" + n); return n + 1; };
function both(c: boolean, o: O) { return c ? f(match (o) { A => 1, B => 2 }) : g(match (o) { A => 3, B => 4 }); }
function left(c: boolean, o: O) { return c ? f(match (o) { A => 1, B => 2 }) : 0; }
function right(c: boolean, o: O) { return c ? match (o) { A => 5, B => 6 } : g(match (o) { A => 7, B => 8 }); }
function wrapped(c: boolean, r: R<number>): R<number> { const v = c ? Ok(try r) : Ok(0); return v; }
function summed(c: boolean, r: R<number>): R<number> { const v = c ? (try r) + 1 : 0; return Ok(v); }
console.log(JSON.stringify([both(true, O.A), both(false, O.B), left(true, O.B), left(false, O.A), right(true, O.B), right(false, O.A), log]));
console.log(JSON.stringify([wrapped(true, Ok(4)), wrapped(true, Err("e")), wrapped(false, Err("e")), summed(true, Ok(4)), summed(true, Err("e")), summed(false, Err("e"))]));

export {};


//// [aConditionalBranchOwnsTheCallsAroundItsValue.ts]
function $tt_raise(error: unknown): never { throw error; }
function $tt_show(value: unknown): string {
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return String(value);
}

type O =
  | { kind: "A" }
  | { kind: "B" };
const O = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const Ok = <T,>(value: T): R<T> => ({ kind: "Ok", value });
const Err = (error: string): R<never> => ({ kind: "Err", error });
const log: string[] = [];
const f = (n: number) => { log.push("f" + n); return n * 10; };
const g = (n: number) => { log.push("g" + n); return n + 1; };
function both(c: boolean, o: O) { let $tt_subject;
let $tt_subject_1;

return c ? f(($tt_subject = o, ($tt_subject.kind === "A") ? 1 : ($tt_subject.kind === "B") ? 2 : $tt_raise(new Error("tt match: unexpected case " + $tt_show($tt_subject))))) : g(($tt_subject_1 = o, ($tt_subject_1.kind === "A") ? 3 : ($tt_subject_1.kind === "B") ? 4 : $tt_raise(new Error("tt match: unexpected case " + $tt_show($tt_subject_1))))); }
function left(c: boolean, o: O) { let $tt_v9: number;
if (c) {
  let $tt_v6: number;
  const $tt_v7 = (f);
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v6 = 1;
        break;
      }
      case "B": {
        $tt_v6 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v9 = $tt_v7($tt_v6);
} else {
  $tt_v9 = 0;
}

return $tt_v9; }
function right(c: boolean, o: O) { let $tt_subject_3;
let $tt_subject_4;

return c ? ($tt_subject_3 = o, ($tt_subject_3.kind === "A") ? 5 : ($tt_subject_3.kind === "B") ? 6 : $tt_raise(new Error("tt match: unexpected case " + $tt_show($tt_subject_3)))) : g(($tt_subject_4 = o, ($tt_subject_4.kind === "A") ? 7 : ($tt_subject_4.kind === "B") ? 8 : $tt_raise(new Error("tt match: unexpected case " + $tt_show($tt_subject_4))))); }
function wrapped(c: boolean, r: R<number>): R<number> { let $tt_v18: R<number>;
if (c) {
  let $tt_v15: number;
  const $tt_v16 = (Ok);
  const $tt_t0 = r;
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v15 = $tt_t0.value;
  $tt_v18 = $tt_v16($tt_v15);
} else {
  $tt_v18 = Ok(0);
}

const v = $tt_v18; return v; }
function summed(c: boolean, r: R<number>): R<number> { let $tt_v21: number;
if (c) {
  let $tt_v19: number;
  const $tt_t1 = r;
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  $tt_v19 = $tt_t1.value;
  $tt_v21 = ($tt_v19) + 1;
} else {
  $tt_v21 = 0;
}

const v = $tt_v21; return Ok(v); }
console.log(JSON.stringify([both(true, O.A), both(false, O.B), left(true, O.B), left(false, O.A), right(true, O.B), right(false, O.A), log]));
console.log(JSON.stringify([wrapped(true, Ok(4)), wrapped(true, Err("e")), wrapped(false, Err("e")), summed(true, Ok(4)), summed(true, Err("e")), summed(false, Err("e"))]));

export {};
