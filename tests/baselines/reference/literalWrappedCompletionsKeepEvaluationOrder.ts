//// [literalWrappedCompletionsKeepEvaluationOrder.tt] ////

variant State { Ready(value: number), Empty }
const trace: string[] = [];
function mark<T>(name: string, value: T): T { trace.push(name); return value; }
type Item = {kind: string; run: (x: number) => number};
function consume(item: Item) { trace.push("call:" + item.kind + ":" + item.run(3)); }
for (const state of [State.Ready(4), State.Empty]) {
  trace.length = 0;
  consume({kind: "inert", run: match (mark("subject", state)) {
    Ready(value) => x => x + value,
    Empty => x => x,
  }});
  console.log(trace.join(","));
}
trace.length = 0;
// An earlier position that observes something keeps the literal where it
// was written, so it still runs before the scrutinee. The arms annotate
// their own parameters here: this case is about order, and the value takes
// the join slot precisely because the literal did not move.
consume({kind: mark("sibling", "effectful"), run: match (mark("subject", State.Ready(1))) {
  Ready(value) => (x: number) => x + value,
  Empty => (x: number) => x,
}});
console.log(trace.join(","));

export {};


//// [literalWrappedCompletionsKeepEvaluationOrder.ts]
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

type State =
  | { kind: "Ready"; value: number }
  | { kind: "Empty" };
const State = {
  Ready: (value: number): State => ({ kind: "Ready", value }),
  Empty: { kind: "Empty" } as const,
};
const trace: string[] = [];
function mark<T>(name: string, value: T): T { trace.push(name); return value; }
type Item = {kind: string; run: (x: number) => number};
function consume(item: Item) { trace.push("call:" + item.kind + ":" + item.run(3)); }
for (const state of [State.Ready(4), State.Empty]) {
  trace.length = 0;
  const $tt_v2: typeof consume = (consume);
  {
    const $tt_m = mark("subject", state);
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        $tt_v2({kind: "inert", run: x => x + value});
        break;
      }
      case "Empty": {
        $tt_v2({kind: "inert", run: x => x});
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  
  console.log(trace.join(","));
}
trace.length = 0;
// An earlier position that observes something keeps the literal where it
// was written, so it still runs before the scrutinee. The arms annotate
// their own parameters here: this case is about order, and the value takes
// the join slot precisely because the literal did not move.
let $tt_v3: (x: number) => number;
const $tt_v5: typeof consume = (consume);
const $tt_v4 = (mark("sibling", "effectful"));
{
  const $tt_m = mark("subject", State.Ready(1));
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v3 = (void 0, (x: number) => x + value);
      break;
    }
    case "Empty": {
      $tt_v3 = (void 0, (x: number) => x);
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
$tt_v5({kind: $tt_v4, run: $tt_v3});
console.log(trace.join(","));

export {};
