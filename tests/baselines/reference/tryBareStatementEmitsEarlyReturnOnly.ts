//// [tryBareStatementEmitsEarlyReturnOnly.tt] ////
function f(): X {
  try g();
  return h();
}


//// [tryBareStatementEmitsEarlyReturnOnly.ts]
function f(): X {
  const $tt_t0 = g();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  return h();
}
