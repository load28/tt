type Reading =
  | { kind: "Value"; n: number }
  | { kind: "Missing" };
const Reading = {
  Value: (n: number): Reading => ({ kind: "Value", n }),
  Missing: { kind: "Missing" } as const,
};
declare const reading: Reading;

export function describe(r: Reading): string {
  let $tt_v0: string;
  {
    const $tt_m = r;
    do {
      if ($tt_m.kind === "Value") {
        const { n } = $tt_m;
        if (n > 100) {
          $tt_v0 = "high";
          break;
        }
      }
      if ($tt_m.kind === "Value") {
        const { n } = $tt_m;
        {
      const label = n.toFixed(1);
      $tt_v0 = `value ${label}`;
      break;
        }
      }
      if ($tt_m.kind === "Missing") {
        $tt_v0 = "missing";
        break;
      }
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    } while (false);
  }
  return $tt_v0;
}
function $tt_show(value: unknown): string {
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return String(value);
}
