//// [typecheckLiteralMatchBlockBodies.tt] ////

const label: string = match ("a" as "a" | "b") {
  "a" => { return "first"; },
  "b" => { return "second"; },
};

export {};


//// [typecheckLiteralMatchBlockBodies.ts]
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

let $tt_v0: string;
{
  const $tt_m = "a" as "a" | "b";
  switch ($tt_m) {
    case "a": {
      $tt_v0 = "first"; break;
    }
    case "b": {
      $tt_v0 = "second"; break;
    }
    default: {
      throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
    }
  }
}
const label: string = $tt_v0;

export {};
