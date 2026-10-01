//// [tryInsideAFunctionInsideAPipelineStepIsAllowed.tt] ////
const a = x |> (n => { const b = try f(n); return b; });


//// [tryInsideAFunctionInsideAPipelineStepIsAllowed.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
const a = $tt_ap(x, ((n => { const $tt_t0 = f(n);
if (!("value" in $tt_t0)) {
  return $tt_t0;
}
const b = $tt_t0.value; return b; })));
