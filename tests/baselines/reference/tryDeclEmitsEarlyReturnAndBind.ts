//// [tryDeclEmitsEarlyReturnAndBind.tt] ////
function f(): X {
  const n = try g();
  return h(n);
}


//// [tryDeclEmitsEarlyReturnAndBind.ts]
function f(): X {
  const $tt_t0 = g();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const n = $tt_t0.value;
  return h(n);
}
