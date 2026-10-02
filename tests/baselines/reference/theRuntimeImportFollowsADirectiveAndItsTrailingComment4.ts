//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment4.tt] ////
"use strict"; 'use client' // x
'b'
declare const o: { p: number };
export const a = o.p |> String;


//// [theRuntimeImportFollowsADirectiveAndItsTrailingComment4.ts]
"use strict"; 'use client' // x
'b'
declare const o: { p: number };
export const a = (($tt_v, $tt_f) => $tt_f($tt_v))(o.p, String);
