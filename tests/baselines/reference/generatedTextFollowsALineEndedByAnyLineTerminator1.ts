//// [generatedTextFollowsALineEndedByAnyLineTerminator1.tt] ////
"use client" // client
declare const o: { p: number };
export const a = o.p |> String;


//// [generatedTextFollowsALineEndedByAnyLineTerminator1.ts]
"use client" // client
declare const o: { p: number };
export const a = (($tt_v, $tt_f) => $tt_f($tt_v))(o.p, String);
