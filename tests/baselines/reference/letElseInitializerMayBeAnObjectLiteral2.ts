//// [letElseInitializerMayBeAnObjectLiteral2.tt] ////
function f(o?: { kind: "Some"; value: number }, c = true) {
  const Some(value) = o ?? (c ? { kind: "Some" as const, value: 1 } : { kind: "None" as const }) else { return 0; };
  const Some(value: w) = c ? { kind: "Some" as const, value } : { kind: "None" as const } else { return 1; };
  return w;
}


//// [letElseInitializerMayBeAnObjectLiteral2.ts]
function f(o?: { kind: "Some"; value: number }, c = true) {
  const $tt_t0 = o ?? (c ? { kind: "Some" as const, value: 1 } : { kind: "None" as const });
  if ($tt_t0.kind !== "Some") {
    return 0;
  }
  const { value } = $tt_t0;
  const $tt_t1 = c ? { kind: "Some" as const, value } : { kind: "None" as const };
  if ($tt_t1.kind !== "Some") {
    return 1;
  }
  const { value: w } = $tt_t1;
  return w;
}
