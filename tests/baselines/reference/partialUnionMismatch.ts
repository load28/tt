//// [partialUnionMismatch.tt] ////
// A value whose union type is only partly incompatible with its target:
// TypeScript names the incompatible part in its elaboration.
declare function toEither(n: number): number | boolean;
declare function needsNumber(n: number): number;
export const direct: number = toEither(1);
export const piped = 1 |> toEither |> needsNumber;
declare function needsTwo(n: number, m: number): number;
export const argument = needsTwo(1, 1 |> toEither);
variant K { A, B }
declare const k: K;
export const matched: number = match (k) { A => toEither(1), B => 2 };

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [partialUnionMismatch.ts]
import { $tt_ap } from "./tt/runtime.js";
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
// A value whose union type is only partly incompatible with its target:
// TypeScript names the incompatible part in its elaboration.
declare function toEither(n: number): number | boolean;
declare function needsNumber(n: number): number;
export const direct: number = toEither(1);
export const piped = $tt_ap(toEither(1), needsNumber);
declare function needsTwo(n: number, m: number): number;
export const argument = needsTwo(1, toEither(1));
type K =
  | { kind: "A" }
  | { kind: "B" };
const K = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
declare const k: K;
let $tt_v4: number;
{
  const $tt_m = k;
  switch ($tt_m.kind) {
    case "A": {
      $tt_v4 = toEither(1);
      break;
    }
    case "B": {
      $tt_v4 = 2;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
export const matched: number = $tt_v4;
