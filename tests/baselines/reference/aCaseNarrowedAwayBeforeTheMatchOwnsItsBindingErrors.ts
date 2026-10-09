//// [aCaseNarrowedAwayBeforeTheMatchOwnsItsBindingErrors.tt] ////
variant Shape { Circle(r: number), Rect(w: number) }
export function f(s: Shape): number {
  if (s.kind === "Rect") return 0;
  return match (s) { Circle(r) => r, Rect(w) => w };
}
export function g(s: Shape): number {
  if (s.kind === "Rect") return 0;
  return match (s) { Circle(r) => r, _ => 0 };
}


//// [aCaseNarrowedAwayBeforeTheMatchOwnsItsBindingErrors.ts]
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
type Shape =
  | { kind: "Circle"; r: number }
  | { kind: "Rect"; w: number };
const Shape = {
  Circle: (r: number): Shape => ({ kind: "Circle", r }),
  Rect: (w: number): Shape => ({ kind: "Rect", w }),
};
export function f(s: Shape): number {
  if (s.kind === "Rect") return 0;
  let $tt_v0: number;
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "Circle": {
        const { r } = $tt_m;
        $tt_v0 = r;
        break;
      }
      case "Rect": {
        const { w } = $tt_m;
        $tt_v0 = w;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}
export function g(s: Shape): number {
  if (s.kind === "Rect") return 0;
  let $tt_v1: number;
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "Circle": {
        const { r } = $tt_m;
        $tt_v1 = r;
        break;
      }
      default: {
        $tt_v1 = 0;
        break;
      }
    }
  }
  return $tt_v1;
}
