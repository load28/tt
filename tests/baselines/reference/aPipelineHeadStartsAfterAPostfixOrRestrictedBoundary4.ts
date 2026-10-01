//// [aPipelineHeadStartsAfterAPostfixOrRestrictedBoundary4.tt] ////
declare const o: number;
function f() {
  return
  o |> String;
}


//// [aPipelineHeadStartsAfterAPostfixOrRestrictedBoundary4.ts]
declare const o: number;
function f() {
  return
  ;(($tt_v, $tt_f) => $tt_f($tt_v))(o, String);
}
