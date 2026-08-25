type Dir =
  | { kind: "North"; dx: number }
  | { kind: "South" };
const Dir = {
  North: (dx: number): Dir => ({ kind: "North", dx }),
  South: { kind: "South" } as const,
};
type Speed =
  | { kind: "Fast"; v: number }
  | { kind: "Slow" };
const Speed = {
  Fast: (v: number): Speed => ({ kind: "Fast", v }),
  Slow: { kind: "Slow" } as const,
};
declare const d: Dir;
declare const s: Speed;

let $tt_v0;
{
  const $tt_m0 = d;
  const $tt_m1 = s;
  do {
    if ($tt_m0.kind === "North" && $tt_m1.kind === "Fast") { const { dx } = $tt_m0; const { v } = $tt_m1; $tt_v0 = dx + v; break; }
    if ($tt_m0.kind === "North" && $tt_m1.kind === "Slow") { const { dx } = $tt_m0; $tt_v0 = dx; break; }
    if ($tt_m0.kind === "South") { $tt_v0 = 0; break; }
    throw new Error("tt match: unexpected case " + JSON.stringify([$tt_m0, $tt_m1]));
  } while (false);
}
export const rating = $tt_v0;
