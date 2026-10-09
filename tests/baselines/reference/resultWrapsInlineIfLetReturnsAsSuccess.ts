//// [resultWrapsInlineIfLetReturnsAsSuccess.tt] ////
variant Item { Some(value: number), None }
const value = result { const item = try read(); if let Some(found) = item { return found; } else { return 0; } };


//// [resultWrapsInlineIfLetReturnsAsSuccess.ts]
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
  const item = $tt_t0.value; {
    const $tt_t1 = item;
    if ($tt_t1.kind === "Some") {
      const { found } = $tt_t1;
      { $tt_v0$value = { kind: "Ok" as const, value: found }; break $tt_v0$value; }
    } else {
      { $tt_v0$value = { kind: "Ok" as const, value: 0 }; break $tt_v0$value; }
    }
  }
}
const value = $tt_v0$value;
