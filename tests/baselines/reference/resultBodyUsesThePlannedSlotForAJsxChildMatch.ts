//// [resultBodyUsesThePlannedSlotForAJsxChildMatch.ttx] ////
import type { TResult } from "@tt/std";
variant E { A, B }
variant F { Yes, No }
declare function step(n: number): number;
declare function fallible(n: number): TResult<number, string>;
export function run(e: E, f: F, n: number): number {
  const value = result {
    const first = try fallible(n);
    const chosen = match (e) { A => 1, B => 2 };
    const view = <section data-value={chosen}>{match (f) {
      Yes => <strong>{chosen |> step}</strong>,
      No => null,
    }}</section>;
    void view;
    return first + chosen;
  };
  return value.kind === "Ok" ? 0 : 1;
}

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [resultBodyUsesThePlannedSlotForAJsxChildMatch.tsx]
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
type E =
  | { kind: "A" }
  | { kind: "B" };
const E = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
type F =
  | { kind: "Yes" }
  | { kind: "No" };
const F = {
  Yes: { kind: "Yes" } as const,
  No: { kind: "No" } as const,
};
declare function step(n: number): number;
declare function fallible(n: number): TResult<number, string>;
export function run(e: E, f: F, n: number): number {
  let $tt_v0: (import("./tt/index.js").TErr<string>) | ({
    kind: "Ok";
    value: number;
});
  $tt_v0: {
    const $tt_t0 = fallible(n);
    if (!("value" in $tt_t0)) {
      $tt_v0 = $tt_t0;
      break $tt_v0;
    }
    const first = $tt_t0.value;
    let $tt_v1: number;
    {
      const $tt_m = e;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v1 = 1;
          break;
        }
        case "B": {
          $tt_v1 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    const chosen = $tt_v1;
    let $tt_v2: number;
    const $tt_v4: typeof chosen = (chosen);
    {
      const $tt_m = f;
      switch ($tt_m.kind) {
        case "Yes": $tt_v2 = 0; break;
        case "No": $tt_v2 = 1; break;
        default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
    const view = <section data-value={$tt_v4}>{($tt_v2 === 0 ? <strong>{(($tt_v, $tt_f) => $tt_f($tt_v))(chosen, step)}</strong> : null)}</section>;
    void view;
    {
      $tt_v0 = { kind: "Ok" as const, value: first + chosen };
      break $tt_v0;
    }
  }
  const value = $tt_v0;
  return value.kind === "Ok" ? 0 : 1;
}
