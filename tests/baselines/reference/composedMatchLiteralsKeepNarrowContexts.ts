//// [composedMatchLiteralsKeepNarrowContexts.tt] ////

declare const flag: boolean;
declare function stringValue(value: "one" | "two"): void;
declare function numberValue(value: 1 | 2): void;
stringValue(match (flag) { true => "one", false => "two" });
numberValue(match (flag) { true => 1, false => 2 });

export {};


//// [composedMatchLiteralsKeepNarrowContexts.ts]
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

declare const flag: boolean;
declare function stringValue(value: "one" | "two"): void;
declare function numberValue(value: 1 | 2): void;
let $tt_v0: number;
const $tt_v1 = (stringValue);
{
  const $tt_m = flag;
  switch ($tt_m) {
    case true: $tt_v0 = 0; break;
    case false: $tt_v0 = 1; break;
    default: throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
  }
}
$tt_v1(($tt_v0 === 0 ? "one" : "two"));
let $tt_v2: number;
const $tt_v3 = (numberValue);
{
  const $tt_m = flag;
  switch ($tt_m) {
    case true: $tt_v2 = 0; break;
    case false: $tt_v2 = 1; break;
    default: throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
  }
}
$tt_v3(($tt_v2 === 0 ? 1 : 2));

export {};
