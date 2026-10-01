//// [aPipelineHeadStartsAfterAPostfixOrRestrictedBoundary2.tt] ////
declare const o: number;
let p: number | undefined
p!
o |> String;


//// [aPipelineHeadStartsAfterAPostfixOrRestrictedBoundary2.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
declare const o: number;
let p: number | undefined
p!
$tt_ap(o, String);
