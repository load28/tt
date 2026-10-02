//// [letElseSharesTryTempCounter.tt] ////
function f(): X {
  const n = try g();
  const Some(v) = h(n) else { return fallback(); };
  return wrap(v);
}


//// [letElseSharesTryTempCounter.ts]
function f(): X {
  const $tt_t0 = g();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const n = $tt_t0.value;
  const $tt_t1 = h(n);
  if ($tt_t1.kind !== "Some") {
    return fallback();
  }
  const { v } = $tt_t1;
  return wrap(v);
}
