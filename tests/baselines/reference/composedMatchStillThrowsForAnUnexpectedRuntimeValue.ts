//// [composedMatchStillThrowsForAnUnexpectedRuntimeValue.tt] ////

function choose(flag: boolean) {
  return [match (flag) { true => 1, false => 2 }];
}
try {
  choose("invalid" as unknown as boolean);
} catch (error) {
  console.log(error instanceof Error ? error.message : String(error));
}

export {};


//// [composedMatchStillThrowsForAnUnexpectedRuntimeValue.ts]
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

function choose(flag: boolean) {
  let $tt_v0: number;
  {
    const $tt_m = flag;
    switch ($tt_m) {
      case true: $tt_v0 = 0; break;
      case false: $tt_v0 = 1; break;
      default: throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
    }
  }
  return [($tt_v0 === 0 ? 1 : 2)];
}
try {
  choose("invalid" as unknown as boolean);
} catch (error) {
  console.log(error instanceof Error ? error.message : String(error));
}

export {};
