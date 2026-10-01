//// [generatedTextFollowsALineEndedByAnyLineTerminator3.tt] ////
"use client" // client declare const o: { p: number };
export const a = o.p |> String;

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [generatedTextFollowsALineEndedByAnyLineTerminator3.ts]
"use client" // client import { $tt_ap } from "./tt/runtime.js";
declare const o: { p: number };
export const a = $tt_ap(o.p, String);
