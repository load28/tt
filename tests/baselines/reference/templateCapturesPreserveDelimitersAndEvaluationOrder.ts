//// [templateCapturesPreserveDelimitersAndEvaluationOrder.tt] ////

variant Shape { Circle(radius: number), Point }
const s = Shape.Circle(5);
const trace: string[] = [];
function before() { trace.push("before"); return 2; }
function arm(n: number) { trace.push("arm"); return n; }
const values = [1].map(x => `${before()}${x}` + match(s) { Circle(radius) => arm(radius), Point => 0 });
console.log(values[0], trace.join(","));
console.log(`${1}${match(s) { Circle(radius) => radius, Point => 0 }}${2}`);

export {};


//// [templateCapturesPreserveDelimitersAndEvaluationOrder.ts]
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
  | { kind: "Circle"; radius: number }
  | { kind: "Point" };
const Shape = {
  Circle: (radius: number): Shape => ({ kind: "Circle", radius }),
  Point: { kind: "Point" } as const,
};
const s = Shape.Circle(5);
const trace: string[] = [];
function before() { trace.push("before"); return 2; }
function arm(n: number) { trace.push("arm"); return n; }
const values = [1].map(x => {
  let $tt_v0: number;
  const $tt_v1 = (`${before()}${x}`);
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "Circle": {
        const { radius } = $tt_m;
        $tt_v0 = arm(radius);
        break;
      }
      case "Point": {
        $tt_v0 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v1 + $tt_v0;
});
console.log(values[0], trace.join(","));
let $tt_v2: number;
{
  const $tt_m = s;
  switch ($tt_m.kind) {
    case "Circle": {
      const { radius } = $tt_m;
      $tt_v2 = radius;
      break;
    }
    case "Point": {
      $tt_v2 = 0;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
console.log(`${1}${$tt_v2}${2}`);

export {};
