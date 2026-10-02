//// [aMatchAfterARegexStatementAfterAnIfCompiles.tt] ////
declare const x: Option<number>;
export function f(s: string) {
  if (!s) return;
  /`/.test(s) && s;
  return match (x) { Some(v) => v, None => 0 };
}


//// [aMatchAfterARegexStatementAfterAnIfCompiles.ts]
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
declare const x: Option<number>;
export function f(s: string) {
  if (!s) return;
  /`/.test(s) && s;
  let $tt_v0;
  {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "Some": {
        const { v } = $tt_m;
        $tt_v0 = v;
        break;
      }
      case "None": {
        $tt_v0 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}
