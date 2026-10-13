//// [aCommentInsideAPipelineOperandIsWrittenOnce.tt] ////
variant O { A }
declare const o: O;
declare const f: () => (x: number) => number;
declare const g: (x: number) => number;
declare const h: (x: number) => (y: number) => number;
declare const a: number;

export const head = match (o) { A => f }// after the head's value
() |> g;
export const step = a |> match (o) { A => f }/* inside the step */() |> g;
export const call = a |> h/* before the argument */(match (o) { A => 1 }) |> g;


//// [aCommentInsideAPipelineOperandIsWrittenOnce.ts]
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
  { kind: "A" };
const O = {
  A: { kind: "A" } as const,
};
declare const o: O;
declare const f: () => (x: number) => number;
declare const g: (x: number) => number;
declare const h: (x: number) => (y: number) => number;
declare const a: number;

let $tt_v0: number;
do {
  let $tt_v7: number;
  let $tt_v1: () => (x: number) => number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v1 = f;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v7 = $tt_v1// after the head's value
();
  $tt_v0 = g($tt_v7);
  break;
} while (false);
export const head = $tt_v0;
let $tt_v2: number;
do {
  const $tt_v8: number = a;
  let $tt_v3: () => (x: number) => number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v3 = f;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const $tt_v9: number = $tt_v3/* inside the step */()($tt_v8);
  $tt_v2 = g($tt_v9);
  break;
} while (false);
export const step = $tt_v2;
let $tt_v4: number;
do {
  const $tt_v10: number = a;
  let $tt_v5: number;
  const $tt_v6: typeof h = (h);
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v5 = 1;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const $tt_v11: number = $tt_v6/* before the argument */($tt_v5)($tt_v10);
  $tt_v4 = g($tt_v11);
  break;
} while (false);
export const call = $tt_v4;
