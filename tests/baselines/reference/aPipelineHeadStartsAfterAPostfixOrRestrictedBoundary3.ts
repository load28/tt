//// [aPipelineHeadStartsAfterAPostfixOrRestrictedBoundary3.tt] ////
declare const o: number;
const k = [1] as const
o |> String;


//// [aPipelineHeadStartsAfterAPostfixOrRestrictedBoundary3.ts]
declare const o: number;
const k = [1] as const
;(($tt_v, $tt_f) => $tt_f($tt_v))(o, String);
