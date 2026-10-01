//// [letElseDivergesWhenTheReturnValueIsAnObjectLiteral1.tt] ////
function f(): number {
  const Some(v) = find() else { return { kind: "Err", error: "no" }; };
  return v;
}


//// [letElseDivergesWhenTheReturnValueIsAnObjectLiteral1.ts]
function f(): number {
  const $tt_t0 = find();
  if ($tt_t0.kind !== "Some") {
    return { kind: "Err", error: "no" };
  }
  const { v } = $tt_t0;
  return v;
}
