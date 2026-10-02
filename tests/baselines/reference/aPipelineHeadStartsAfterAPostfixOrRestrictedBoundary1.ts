//// [aPipelineHeadStartsAfterAPostfixOrRestrictedBoundary1.tt] ////
declare const o: number;
let q = 1
q++
o |> String;


//// [aPipelineHeadStartsAfterAPostfixOrRestrictedBoundary1.ts]
declare const o: number;
let q = 1
q++
;(($tt_v, $tt_f) => $tt_f($tt_v))(o, String);
