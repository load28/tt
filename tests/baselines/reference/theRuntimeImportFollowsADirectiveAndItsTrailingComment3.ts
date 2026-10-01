//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment3.tt] ////
"use client" /* a
 b */
declare const o: { p: number };
export const a = o.p |> String;


//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment3.ts]
"use client" /* a
 b */
declare const o: { p: number };
export const a = (($tt_v, $tt_f) => $tt_f($tt_v))(o.p, String);
