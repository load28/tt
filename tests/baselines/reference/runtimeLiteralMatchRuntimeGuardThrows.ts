//// [runtimeLiteralMatchRuntimeGuardThrows.tt] ////

function label(dir: string) {
  return match (dir as "a" | "b") {
    "a" => 1,
    "b" => 2,
  };
}

try {
  label("zzz");
  console.log("no throw");
} catch (e) {
  console.log((e as Error).message);
}

export {};


//// [runtimeLiteralMatchRuntimeGuardThrows.ts]
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

function label(dir: string) {
  let $tt_v0: number;
  {
    const $tt_m = dir as "a" | "b";
    switch ($tt_m) {
      case "a": {
        $tt_v0 = 1;
        break;
      }
      case "b": {
        $tt_v0 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}

try {
  label("zzz");
  console.log("no throw");
} catch (e) {
  console.log((e as Error).message);
}

export {};
