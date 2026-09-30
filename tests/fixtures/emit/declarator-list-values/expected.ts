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
  | { kind: "Square"; side: number };
const Shape = {
  Circle: (r: number): Shape => ({ kind: "Circle", r }),
  Square: (side: number): Shape => ({ kind: "Square", side }),
};

declare function scale(): number;

export function area(shape: Shape) {
  const k = scale();
  let $tt_v0: number;
  {
    const $tt_m = shape;
    switch ($tt_m.kind) {
      case "Circle": {
        const { r } = $tt_m;
        $tt_v0 = k * r * r;
        break;
      }
      case "Square": {
        const { side } = $tt_m;
        $tt_v0 = k * side * side;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const value = $tt_v0;
  return value;
}

export const unit = scale();
  // the second declarator reads the first
let $tt_v1: number;
{
  const $tt_m = Shape.Circle(unit);
  switch ($tt_m.kind) {
    case "Circle": {
      const { r } = $tt_m;
      $tt_v1 = r * 2;
      break;
    }
    case "Square": {
      const { side } = $tt_m;
      $tt_v1 = side;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
export const doubled = $tt_v1,
  label = `${doubled}`;
