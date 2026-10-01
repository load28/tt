//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment6.tt] ////
"use client"
variant V { A, B }
declare const o: { p: number };
export const a = o.p |> String;


//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment6.ts]
"use client"
type V =
  | { kind: "A" }
  | { kind: "B" };
const V = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
declare const o: { p: number };
export const a = (($tt_v, $tt_f) => $tt_f($tt_v))(o.p, String);
