//// [letElseInitializerMayBeAnObjectLiteral1.tt] ////
function f(n: number) {
  const Some(value: v) = { kind: "Some" as const, value: n } else { return; };
  return v;
}


//// [letElseInitializerMayBeAnObjectLiteral1.ts]
function f(n: number) {
  const $tt_t0 = { kind: "Some" as const, value: n };
  if ($tt_t0.kind !== "Some") {
    return;
  }
  const { value: v } = $tt_t0;
  return v;
}
