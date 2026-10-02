//// [aGroupedValueStillEvaluatesToWhatTheArmWrote.tt] ////
variant E { A(v: number), B }
const e: E = E.A(1);
const seen: number[] = [];
const note = (n: number): number => { seen.push(n); return n; };
const seq = match (e) {
A(v) => {
return note(v), v + 10;
},
B => 0,
};
const width = 3;
const receiver = width + 0.5 |> .toFixed(1);
const chained = "  pad  " |> .trim() |> .length;
console.log(seq, seen.join(","), receiver, chained);

export {};


//// [aGroupedValueStillEvaluatesToWhatTheArmWrote.ts]
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
type E =
  | { kind: "A"; v: number }
  | { kind: "B" };
const E = {
  A: (v: number): E => ({ kind: "A", v }),
  B: { kind: "B" } as const,
};
const e: E = E.A(1);
const seen: number[] = [];
const note = (n: number): number => { seen.push(n); return n; };
let $tt_v0: number;
{
  const $tt_m = e;
  switch ($tt_m.kind) {
    case "A": {
      const { v } = $tt_m;
      $tt_v0 = (note(v), v + 10);
  break;
    }
    case "B": {
      $tt_v0 = 0;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const seq = $tt_v0;
const width = 3;
const receiver = (width + 0.5).toFixed(1);
const chained = "  pad  ".trim().length;
console.log(seq, seen.join(","), receiver, chained);

export {};
