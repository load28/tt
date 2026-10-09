//// [anExportedLetElseKeepsItsDocumentationOnTheBinding.tt] ////
variant O { A(n: number), B, C(n: number) }
declare const o: O;
/** the count */
export const x = match (o) { A(n) => n, B => 0, C(n) => n };
/** binding doc */
export const A(n: z) = o else { throw 0; };
/** or doc */
export const A(n: q) | C(n: q) = o else { throw 0; };


//// [anExportedLetElseKeepsItsDocumentationOnTheBinding.ts]
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
  | { kind: "A"; n: number }
  | { kind: "B" }
  | { kind: "C"; n: number };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
  C: (n: number): O => ({ kind: "C", n }),
};
declare const o: O;
let $tt_v0: number;
{
  const $tt_m = o;
  switch ($tt_m.kind) {
    case "A": {
      const { n } = $tt_m;
      $tt_v0 = n;
      break;
    }
    case "B": {
      $tt_v0 = 0;
      break;
    }
    case "C": {
      const { n } = $tt_m;
      $tt_v0 = n;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
/** the count */
export const x = $tt_v0;
const $tt_t0 = o;
if ($tt_t0.kind !== "A") {
  throw 0;
}
/** binding doc */
export const { n: z } = $tt_t0;
const $tt_t1 = o;
if ($tt_t1.kind !== "A" && $tt_t1.kind !== "C") {
  throw 0;
}
/** or doc */
export const { n: q } = $tt_t1;
