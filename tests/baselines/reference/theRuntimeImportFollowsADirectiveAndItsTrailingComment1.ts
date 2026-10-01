//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment1.tt] ////
"use client" // client component
declare const o: { p: number };
export const a = o.p |> String;

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment1.ts]
"use client" // client component
import { $tt_ap } from "./tt/runtime.js";
declare const o: { p: number };
export const a = $tt_ap(o.p, String);
