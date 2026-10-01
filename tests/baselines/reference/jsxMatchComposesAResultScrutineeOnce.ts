//// [jsxMatchComposesAResultScrutineeOnce.ttx] ////
import type { TResult } from "@tt/std";
declare const outcome: TResult<string, string>;
const view = <aside>{match (result {
  const value = try outcome;
  return value |> .toUpperCase();
}) {
  Ok(value) => <b>{value}</b>,
  Err(error) => <code>{error}</code>,
}}</aside>;

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [jsxMatchComposesAResultScrutineeOnce.tsx]
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
import type { TResult } from "./tt/index.js";
declare const outcome: TResult<string, string>;
let $tt_v0;
{
  let $tt_m; $tt_y_v1: {
    const $tt_t0 = outcome;
    if (!("value" in $tt_t0)) {
      $tt_m = $tt_t0;
      break $tt_y_v1;
    }
    const value = $tt_t0.value;
  {
    $tt_m = { kind: "Ok" as const, value: value.toUpperCase() };
    break $tt_y_v1;
  }
  }
  switch ($tt_m.kind) {
    case "Ok": {
      const { value } = $tt_m;
      $tt_v0 = <b>{value}</b>;
      break;
    }
    case "Err": {
      const { error } = $tt_m;
      $tt_v0 = <code>{error}</code>;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const view = <aside>{$tt_v0}</aside>;
