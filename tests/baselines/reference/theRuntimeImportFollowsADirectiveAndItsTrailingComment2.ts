//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment2.tt] ////
"use client" /* c */;
declare const o: { p: number };
export const a = o.p |> String;

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment2.ts]
"use client" /* c */;
import { $tt_ap } from "./tt/runtime.js";
declare const o: { p: number };
export const a = $tt_ap(o.p, String);
