//// [aPipelineHeadStartsAfterAPostfixOrRestrictedBoundary3.tt] ////
declare const o: number;
const k = [1] as const
o |> String;


//// [aPipelineHeadStartsAfterAPostfixOrRestrictedBoundary3.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
declare const o: number;
const k = [1] as const
$tt_ap(o, String);
