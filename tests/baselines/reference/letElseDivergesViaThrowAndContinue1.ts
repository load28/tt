//// [letElseDivergesViaThrowAndContinue1.tt] ////
function f(): number {
  const Some(v) = find() else { throw new Error("no"); };
  return v;
}


//// [letElseDivergesViaThrowAndContinue1.ts]
function f(): number {
  const $tt_t0 = find();
  if ($tt_t0.kind !== "Some") {
    throw new Error("no");
  }
  const { v } = $tt_t0;
  return v;
}
