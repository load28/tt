//// [aPipelineHeadStartsAfterAPostfixOrRestrictedBoundary2.tt] ////
declare const o: number;
let p: number | undefined
p!
o |> String;


//// [aPipelineHeadStartsAfterAPostfixOrRestrictedBoundary2.ts]
declare const o: number;
let p: number | undefined
p!
;(($tt_v, $tt_f) => $tt_f($tt_v))(o, String);
