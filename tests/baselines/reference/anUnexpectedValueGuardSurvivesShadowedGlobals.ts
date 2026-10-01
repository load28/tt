//// [anUnexpectedValueGuardSurvivesShadowedGlobals.tt] ////

const String = "shadow";
const JSON = 1;
function pick(n: unknown): number {
  return match (n) { 1n => 1 };
}
for (const value of [2n, Symbol("s"), 3, "a", { a: 1 }]) {
  try { pick(value); } catch (error) { console.log((error as Error).message); }
}
console.log(String, JSON);

export {};


//// [anUnexpectedValueGuardSurvivesShadowedGlobals.ts]
function $tt_show(value: unknown): string {
  if (typeof value === "string") {
    return globalThis.JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return globalThis.String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = globalThis.JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return globalThis.String(value);
}

const String = "shadow";
const JSON = 1;
function pick(n: unknown): number {
  let $tt_v0: number;
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 1n: {
        $tt_v0 = 1;
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}
for (const value of [2n, Symbol("s"), 3, "a", { a: 1 }]) {
  try { pick(value); } catch (error) { console.log((error as Error).message); }
}
console.log(String, JSON);

export {};
