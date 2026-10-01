//// [letElseDivergesViaThrowAndContinue2.tt] ////
function f(): number {
  for (const x of xs) {
    const Some(v) = find(x) else { continue; };
    use(v);
  }
  return 0;
}


//// [letElseDivergesViaThrowAndContinue2.ts]
function f(): number {
  for (const x of xs) {
    const $tt_t0 = find(x);
    if ($tt_t0.kind !== "Some") {
      continue;
    }
    const { v } = $tt_t0;
    use(v);
  }
  return 0;
}
