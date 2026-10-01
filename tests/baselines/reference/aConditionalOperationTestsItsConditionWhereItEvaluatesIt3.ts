//// [aConditionalOperationTestsItsConditionWhereItEvaluatesIt3.tt] ////
variant O { A(n: number), B }
declare const o: O;
declare const cfg: { name?: string };
export const v = !cfg.name || match (o) { A(n) => n, B => 0 };


//// [aConditionalOperationTestsItsConditionWhereItEvaluatesIt3.ts]
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
  | { kind: "B" };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
declare const o: O;
declare const cfg: { name?: string };
let $tt_v2: (true) | (number);
let $tt_v1: boolean;
if ($tt_v1 = !cfg.name) {
  $tt_v2 = $tt_v1;
} else {
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
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v2 = $tt_v1 || $tt_v0;
}

export const v = $tt_v2;
