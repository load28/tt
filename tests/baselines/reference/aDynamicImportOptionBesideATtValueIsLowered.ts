//// [other.ts] ////
export const value = 1;

//// [main.tt] ////
variant V { A, B }
const v = V.A as V;
export async function load() {
  return await import("./other", match (v) { A => ({ with: {} }), B => undefined });
}


//// [main.ts]
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
type V =
  | { kind: "A" }
  | { kind: "B" };
const V = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const v = V.A as V;
export async function load() {
  let $tt_v0: number;
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": $tt_v0 = 0; break;
      case "B": $tt_v0 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  return await import("./other", ($tt_v0 === 0 ? ({ with: {} }) : undefined));
}

//// [other.ts]
export const value = 1;
\ No newline at end of file
