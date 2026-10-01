//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment2.tt] ////
"use client" /* c */;
declare const o: { p: number };
export const a = o.p |> String;


//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment2.ts]
"use client" /* c */;
declare const o: { p: number };
export const a = (($tt_v, $tt_f) => $tt_f($tt_v))(o.p, String);
