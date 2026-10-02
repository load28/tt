//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment1.tt] ////
"use client" // client component
declare const o: { p: number };
export const a = o.p |> String;


//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment1.ts]
"use client" // client component
declare const o: { p: number };
export const a = (($tt_v, $tt_f) => $tt_f($tt_v))(o.p, String);
