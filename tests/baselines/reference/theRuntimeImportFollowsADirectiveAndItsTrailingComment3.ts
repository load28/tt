//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment3.tt] ////
"use client" /* a
 b */
declare const o: { p: number };
export const a = o.p |> String;

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment3.ts]
"use client" /* a
 b */
import { $tt_ap } from "./tt/runtime.js";
declare const o: { p: number };
export const a = $tt_ap(o.p, String);
