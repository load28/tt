//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment5.tt] ////
"use client";declare const o: { p: number };
export const a = o.p |> String;


//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment5.ts]
"use client";declare const o: { p: number };
export const a = (($tt_v, $tt_f) => $tt_f($tt_v))(o.p, String);
