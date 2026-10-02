//// [runtimeLiteralBooleanMatch.tt] ////

function label(flag: boolean) {
  return match (flag) {
    true => "yes",
    false => "no",
  };
}

console.log(label(true), label(false));

export {};


//// [runtimeLiteralBooleanMatch.ts]
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

function label(flag: boolean) {
  let $tt_v0: string;
  {
    const $tt_m = flag;
    switch ($tt_m) {
      case true: {
        $tt_v0 = "yes";
        break;
      }
      case false: {
        $tt_v0 = "no";
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}

console.log(label(true), label(false));

export {};
