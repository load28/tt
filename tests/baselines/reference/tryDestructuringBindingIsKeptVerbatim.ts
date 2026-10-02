//// [tryDestructuringBindingIsKeptVerbatim.tt] ////
function f(): X {
  const { a, b } = try g();
  return a + b;
}


//// [tryDestructuringBindingIsKeptVerbatim.ts]
function f(): X {
  const $tt_t0 = g();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const { a, b } = $tt_t0.value;
  return a + b;
}
