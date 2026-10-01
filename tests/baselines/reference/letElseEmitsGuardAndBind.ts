//// [letElseEmitsGuardAndBind.tt] ////
function f(): number {
  const Some(value) = find() else { return 0; };
  return value;
}


//// [letElseEmitsGuardAndBind.ts]
function f(): number {
  const $tt_t0 = find();
  if ($tt_t0.kind !== "Some") {
    return 0;
  }
  const { value } = $tt_t0;
  return value;
}
