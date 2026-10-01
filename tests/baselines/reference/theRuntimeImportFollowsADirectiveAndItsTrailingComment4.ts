//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment4.tt] ////
"use strict"; 'use client' // x
'b'
declare const o: { p: number };
export const a = o.p |> String;

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment4.ts]
"use strict"; 'use client' // x
'b'
import { $tt_ap } from "./tt/runtime.js";
declare const o: { p: number };
export const a = $tt_ap(o.p, String);
