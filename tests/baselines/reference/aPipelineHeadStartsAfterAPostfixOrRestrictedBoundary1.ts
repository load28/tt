//// [aPipelineHeadStartsAfterAPostfixOrRestrictedBoundary1.tt] ////
declare const o: number;
let q = 1
q++
o |> String;


//// [aPipelineHeadStartsAfterAPostfixOrRestrictedBoundary1.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
declare const o: number;
let q = 1
q++
$tt_ap(o, String);
