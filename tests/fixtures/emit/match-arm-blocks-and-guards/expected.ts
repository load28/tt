type Reading =
  | { kind: "Value"; n: number }
  | { kind: "Missing" };
const Reading = {
  Value: (n: number): Reading => ({ kind: "Value", n }),
  Missing: { kind: "Missing" } as const,
};
declare const reading: Reading;

export function describe(r: Reading): string {
  let $tt_v0;
  {
    const $tt_m = r;
    do {
      if ($tt_m.kind === "Value") { const { n } = $tt_m; if (n > 100) { $tt_v0 = "high"; break; } }
      if ($tt_m.kind === "Value") { const { n } = $tt_m; {
      const label = n.toFixed(1);
      $tt_v0 = `value ${label}`; break;
      } }
      if ($tt_m.kind === "Missing") { $tt_v0 = "missing"; break; }
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    } while (false);
  }
  return $tt_v0;
}
