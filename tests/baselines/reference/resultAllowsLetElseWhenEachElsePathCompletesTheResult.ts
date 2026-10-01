//// [resultAllowsLetElseWhenEachElsePathCompletesTheResult.tt] ////
variant Item { Some(value: number), None }
const value = result { const item = try read(); let Some(found) = item else { return 0; }; return found; };


//// [resultAllowsLetElseWhenEachElsePathCompletesTheResult.ts]
type Item =
  | { kind: "Some"; value: number }
  | { kind: "None" };
const Item = {
  Some: (value: number): Item => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};
let $tt_v0$value;
$tt_v0$value: {
  const $tt_t0 = read();
  if (!("value" in $tt_t0)) {
    $tt_v0$value = $tt_t0;
    break $tt_v0$value;
  }
  const item = $tt_t0.value; const $tt_t1 = item;
  if ($tt_t1.kind !== "Some") {
    { const $tt_a0 = { value: { kind: "Ok" as const, value: 0 } }; $tt_v0$value = $tt_a0.value; break $tt_v0$value; }
  }
  let { found } = $tt_t1; { const $tt_a1 = { value: { kind: "Ok" as const, value: found } }; $tt_v0$value = $tt_a1.value; break $tt_v0$value; }
}
const value = $tt_v0$value;
