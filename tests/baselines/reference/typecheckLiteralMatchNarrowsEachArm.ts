//// [typecheckLiteralMatchNarrowsEachArm.tt] ////

type Size = "sm" | "md" | "lg";
const px: number = match ("sm" as Size) {
  "sm" => 12,
  "md" => 16,
  "lg" => 20,
};

export {};


//// [typecheckLiteralMatchNarrowsEachArm.ts]
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

type Size = "sm" | "md" | "lg";
let $tt_v0: number;
{
  const $tt_m = "sm" as Size;
  switch ($tt_m) {
    case "sm": {
      $tt_v0 = 12;
      break;
    }
    case "md": {
      $tt_v0 = 16;
      break;
    }
    case "lg": {
      $tt_v0 = 20;
      break;
    }
    default: {
      throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
    }
  }
}
const px: number = $tt_v0;

export {};
