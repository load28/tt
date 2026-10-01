//// [tryTemporariesAreUniqueAndKeepDeclarationKeyword.tt] ////
function f(): X {
  let a: number = try g();
  var b = try h(a);
  return k(b);
}


//// [tryTemporariesAreUniqueAndKeepDeclarationKeyword.ts]
function f(): X {
  const $tt_t0 = g();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  let a: number = $tt_t0.value;
  const $tt_t1 = h(a);
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  var b = $tt_t1.value;
  return k(b);
}
