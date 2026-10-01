//// [pipelineComposesWithTry.tt] ////
function f(): Result<number, string> {
  const a = try readCfg() |> norm;
  return Result.Ok(a);
}


//// [pipelineComposesWithTry.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
function f(): Result<number, string> {
  const $tt_t0 = $tt_ap(readCfg(), norm);
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const a = $tt_t0.value;
  return Result.Ok(a);
}
