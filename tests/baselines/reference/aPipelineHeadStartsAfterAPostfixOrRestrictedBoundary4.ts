//// [aPipelineHeadStartsAfterAPostfixOrRestrictedBoundary4.tt] ////
declare const o: number;
function f() {
  return
  o |> String;
}


//// [aPipelineHeadStartsAfterAPostfixOrRestrictedBoundary4.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
declare const o: number;
function f() {
  return
  $tt_ap(o, String);
}
