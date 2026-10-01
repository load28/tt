//// [letElseDivergesWhenTheReturnValueIsAnObjectLiteral2.tt] ////
function f(): number {
  const Some(v) = find() else { log("x"); return { k: 1 }; };
  return v;
}


//// [letElseDivergesWhenTheReturnValueIsAnObjectLiteral2.ts]
function f(): number {
  const $tt_t0 = find();
  if ($tt_t0.kind !== "Some") {
    log("x"); return { k: 1 };
  }
  const { v } = $tt_t0;
  return v;
}
