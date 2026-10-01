//// [letElseDivergesWhenTheReturnValueIsAnObjectLiteral4.tt] ////
function f(): number {
  const Some(v) = find() else { throw { code: 1 }; };
  return v;
}


//// [letElseDivergesWhenTheReturnValueIsAnObjectLiteral4.ts]
function f(): number {
  const $tt_t0 = find();
  if ($tt_t0.kind !== "Some") {
    throw { code: 1 };
  }
  const { v } = $tt_t0;
  return v;
}
