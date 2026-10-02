//// [letElseEmptyBindingsChecksOnly.tt] ////
function f(): number {
  const Ok() = check() else { return -1; };
  return 1;
}


//// [letElseEmptyBindingsChecksOnly.ts]
function f(): number {
  const $tt_t0 = check();
  if ($tt_t0.kind !== "Ok") {
    return -1;
  }
  
  return 1;
}
