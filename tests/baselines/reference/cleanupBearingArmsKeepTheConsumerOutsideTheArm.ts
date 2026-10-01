//// [cleanupBearingArmsKeepTheConsumerOutsideTheArm.tt] ////

variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
const trace: string[] = [];
function consume(item: Item): number { trace.push("call"); return item.run(1); }
const finalized = consume(match (State.Ready(3)) {
  Ready(value) => {
    try {
      trace.push("arm");
      return {kind: "item" as const, run: (x: number) => x + value};
    } finally {
      trace.push("finally");
    }
  },
  Empty => ({kind: "item" as const, run: (x: number) => x}),
});
console.log(finalized, trace.join(","));
trace.length = 0;
function throwingConsumer(item: Item): number { trace.push("throwing"); throw new Error("consumer"); }
try {
  throwingConsumer(match (State.Ready(2)) {
    Ready(value) => {
      try {
        return {kind: "item" as const, run: (x: number) => x + value};
      } catch {
        trace.push("handler");
        return {kind: "item" as const, run: (x: number) => x};
      }
    },
    Empty => ({kind: "item" as const, run: (x: number) => x}),
  });
} catch { trace.push("caught outside"); }
console.log(trace.join(","));

export {};


//// [cleanupBearingArmsKeepTheConsumerOutsideTheArm.ts]
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
type Item = {kind: "item"; run: (x: number) => number};
const trace: string[] = [];
function consume(item: Item): number { trace.push("call"); return item.run(1); }
let $tt_v0: Item;
const $tt_v1 = (consume);
{
  const $tt_m = State.Ready(3);
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      try {
      trace.push("arm");
        $tt_v0 = {kind: "item" as const, run: (x: number) => x + value};
        break;
    } finally {
      trace.push("finally");
    }
    }
    case "Empty": {
      $tt_v0 = ({kind: "item" as const, run: (x: number) => x});
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const finalized = $tt_v1($tt_v0);
console.log(finalized, trace.join(","));
trace.length = 0;
function throwingConsumer(item: Item): number { trace.push("throwing"); throw new Error("consumer"); }
try {
  let $tt_v2: Item;
  const $tt_v3 = (throwingConsumer);
  {
    const $tt_m = State.Ready(2);
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        try {
          $tt_v2 = {kind: "item" as const, run: (x: number) => x + value};
          break;
      } catch {
        trace.push("handler");
          $tt_v2 = {kind: "item" as const, run: (x: number) => x};
          break;
      }
      }
      case "Empty": {
        $tt_v2 = ({kind: "item" as const, run: (x: number) => x});
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v3($tt_v2);
} catch { trace.push("caught outside"); }
console.log(trace.join(","));

export {};
