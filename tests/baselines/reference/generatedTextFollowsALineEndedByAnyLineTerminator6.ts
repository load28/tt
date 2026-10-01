//// [generatedTextFollowsALineEndedByAnyLineTerminator6.tt] ////
#!/usr/bin/env node declare const o: { p: number };
export const a = o.p |> String;

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [generatedTextFollowsALineEndedByAnyLineTerminator6.ts]
#!/usr/bin/env node import { $tt_ap } from "./tt/runtime.js";
declare const o: { p: number };
export const a = $tt_ap(o.p, String);
