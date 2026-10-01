//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment6.tt] ////
"use client"
variant V { A, B }
declare const o: { p: number };
export const a = o.p |> String;

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment6.ts]
"use client"
import { $tt_ap } from "./tt/runtime.js";
type V =
  | { kind: "A" }
  | { kind: "B" };
const V = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
declare const o: { p: number };
export const a = $tt_ap(o.p, String);
