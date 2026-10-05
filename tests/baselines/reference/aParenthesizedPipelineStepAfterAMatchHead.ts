//// [aParenthesizedPipelineStepAfterAMatchHead.tt] ////
// A parenthesized pipeline step is an ordinary expression inside a pipeline
// whose head is a match. The match is evaluated into the pipeline's storage,
// and the inner pipeline is the step's function, applied to it: the step
// has no statement form of its own to write a value slot through.
variant G { E, F }
const add = (n: number) => (m: number) => n + m;
const p = 10;
function pick(g: G): number {
  return match (g) { E => 1, F => 2 } |> (p |> add);
}
console.log(pick(G.E), pick(G.F));
export {};


//// [aParenthesizedPipelineStepAfterAMatchHead.ts]
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
// A parenthesized pipeline step is an ordinary expression inside a pipeline
// whose head is a match. The match is evaluated into the pipeline's storage,
// and the inner pipeline is the step's function, applied to it: the step
// has no statement form of its own to write a value slot through.
type G =
  | { kind: "E" }
  | { kind: "F" };
const G = {
  E: { kind: "E" } as const,
  F: { kind: "F" } as const,
};
const add = (n: number) => (m: number) => n + m;
const p = 10;
function pick(g: G): number {
  let $tt_v0: number;
  do {
    let $tt_v3: number;
    {
      const $tt_m = g;
      switch ($tt_m.kind) {
        case "E": {
          $tt_v3 = 1;
          break;
        }
        case "F": {
          $tt_v3 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v0 = ((($tt_v, $tt_f) => $tt_f($tt_v))(p, add))($tt_v3);
    break;
  } while (false);
  return $tt_v0;
}
console.log(pick(G.E), pick(G.F));
export {};
