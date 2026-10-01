//// [ifLetSharesTheTempCounterWithTry.tt] ////
function f(): Result<number, string> {
  const a = try g();
  if let Some(value) = h(a) { use(value); }
  return Result.Ok(a);
}


//// [ifLetSharesTheTempCounterWithTry.ts]
function f(): Result<number, string> {
  const $tt_t0 = g();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const a = $tt_t0.value;
  {
    const $tt_t1 = h(a);
    if ($tt_t1.kind === "Some") {
      const { value } = $tt_t1;
      use(value);
    }
  }
  return Result.Ok(a);
}
