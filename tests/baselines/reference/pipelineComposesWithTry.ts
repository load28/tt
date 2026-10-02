//// [pipelineComposesWithTry.tt] ////
function f(): Result<number, string> {
  const a = try readCfg() |> norm;
  return Result.Ok(a);
}


//// [pipelineComposesWithTry.ts]
function f(): Result<number, string> {
  const $tt_t0 = (($tt_v, $tt_f) => $tt_f($tt_v))(readCfg(), norm);
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const a = $tt_t0.value;
  return Result.Ok(a);
}
