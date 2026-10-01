//// [aHostGlobalAliasIsCapturedOnlyWhenGlobalThisIsShadowed4.tt] ////
variant O { Some(value: number), None }
export function f(o: O) { const Error = 5; return match (o) { Some(value) => value, None => 0 }; }


//// [aHostGlobalAliasIsCapturedOnlyWhenGlobalThisIsShadowed4.ts]
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
type O =
  | { kind: "Some"; value: number }
  | { kind: "None" };
const O = {
  Some: (value: number): O => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};
export function f(o: O) { const Error = 5; let $tt_v0: number;
{
  const $tt_m = o;
  switch ($tt_m.kind) {
    case "Some": {
      const { value } = $tt_m;
      $tt_v0 = value;
      break;
    }
    case "None": {
      $tt_v0 = 0;
      break;
    }
    default: {
      throw new globalThis.Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
return $tt_v0; }
