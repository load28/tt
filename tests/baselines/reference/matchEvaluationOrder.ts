//// [matchEvaluationOrder.tt] ////
// A match evaluates its scrutinee once, tests guards in arm order, runs one
// arm, and keeps its place in the evaluation order of the expression around it.
variant Shape { Circle(r: number), Rect(w: number, h: number), Point }
const log: string[] = [];
function note<T>(label: string, value: T): T {
  log.push(label);
  return value;
}
function flush(title: string, value: unknown) {
  console.log(`${title}: ${JSON.stringify(value)} after ${log.join(", ")}`);
  log.length = 0;
}
const shapes = [Shape.Circle(1), Shape.Circle(2), Shape.Rect(3, 2), Shape.Rect(1, 4), Shape.Point];
for (const shape of shapes) {
  const value = match (note("scrutinee", shape)) {
    Circle(r) if note("guard r > 1", r > 1) => note("big circle", r),
    Circle(r) => note("circle", r * 10),
    Rect(w, h) if note("guard w > h", w > h) => note("wide rect", w),
    Rect(w, h) => note("rect", w * h),
    Point => note("point", 0),
  };
  flush(JSON.stringify(shape), value);
}
function triple(a: number, b: number, c: number) {
  return [a, b, c];
}
flush(
  "arguments",
  triple(note("argument 0", 1), match (note("argument 1 scrutinee", shapes[2])) { Rect(w) => note("argument 1 arm", w), _ => 0 }, note("argument 2", 3)),
);
flush(
  "operands",
  note("left", 1) + match (note("right scrutinee", shapes[1])) { Circle(r) => note("right arm", r), _ => 0 } * note("factor", 10),
);
const box = { v: 1 };
function target() {
  log.push("target");
  return box;
}
target().v += match (note("assigned scrutinee", shapes[4])) { Point => note("assigned arm", (box.v = 100, 5)), _ => 0 };
flush("compound assignment", box.v);
const nested = match (match (note("inner scrutinee", shapes[3])) { Rect(w, h) => note("inner arm", w < h), _ => false }) {
  true => note("outer true", "tall"),
  false => note("outer false", "flat"),
};
flush("nested", nested);
const text = `${note("before", "<")}${match (note("template scrutinee", shapes[0])) { Circle(r) => note("template arm", r), _ => 0 }}${note("after", ">")}`;
flush("template", text);
export {};


//// [matchEvaluationOrder.ts]
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
// A match evaluates its scrutinee once, tests guards in arm order, runs one
// arm, and keeps its place in the evaluation order of the expression around it.
type Shape =
  | { kind: "Circle"; r: number }
  | { kind: "Rect"; w: number; h: number }
  | { kind: "Point" };
const Shape = {
  Circle: (r: number): Shape => ({ kind: "Circle", r }),
  Rect: (w: number, h: number): Shape => ({ kind: "Rect", w, h }),
  Point: { kind: "Point" } as const,
};
const log: string[] = [];
function note<T>(label: string, value: T): T {
  log.push(label);
  return value;
}
function flush(title: string, value: unknown) {
  console.log(`${title}: ${JSON.stringify(value)} after ${log.join(", ")}`);
  log.length = 0;
}
const shapes = [Shape.Circle(1), Shape.Circle(2), Shape.Rect(3, 2), Shape.Rect(1, 4), Shape.Point];
for (const shape of shapes) {
  let $tt_v0: number;
  {
    const $tt_m = note("scrutinee", shape);
    do {
      if ($tt_m.kind === "Circle") {
        const { r } = $tt_m;
        if (note("guard r > 1", r > 1)) {
          $tt_v0 = note("big circle", r);
          break;
        }
      }
      if ($tt_m.kind === "Circle") {
        const { r } = $tt_m;
        $tt_v0 = note("circle", r * 10);
        break;
      }
      if ($tt_m.kind === "Rect") {
        const { w, h } = $tt_m;
        if (note("guard w > h", w > h)) {
          $tt_v0 = note("wide rect", w);
          break;
        }
      }
      if ($tt_m.kind === "Rect") {
        const { w, h } = $tt_m;
        $tt_v0 = note("rect", w * h);
        break;
      }
      if ($tt_m.kind === "Point") {
        $tt_v0 = note("point", 0);
        break;
      }
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    } while (false);
  }
  const value = $tt_v0;
  flush(JSON.stringify(shape), value);
}
function triple(a: number, b: number, c: number) {
  return [a, b, c];
}
let $tt_v1: number;
const $tt_v4 = (flush);
const $tt_v2 = (triple);
const $tt_v3 = (note("argument 0", 1));
{
  const $tt_m = note("argument 1 scrutinee", shapes[2]);
  switch ($tt_m.kind) {
    case "Rect": {
      const { w } = $tt_m;
      $tt_v1 = note("argument 1 arm", w);
      break;
    }
    default: {
      $tt_v1 = 0;
      break;
    }
  }
}
$tt_v4(
  "arguments",
  $tt_v2($tt_v3, $tt_v1, note("argument 2", 3)),
);
let $tt_v6: number;
const $tt_v8 = (flush);
const $tt_v7 = (note("left", 1));
{
  const $tt_m = note("right scrutinee", shapes[1]);
  switch ($tt_m.kind) {
    case "Circle": {
      const { r } = $tt_m;
      $tt_v6 = note("right arm", r);
      break;
    }
    default: {
      $tt_v6 = 0;
      break;
    }
  }
}
$tt_v8(
  "operands",
  $tt_v7 + $tt_v6 * note("factor", 10),
);
const box = { v: 1 };
function target() {
  log.push("target");
  return box;
}
let $tt_v10: number;
const $tt_v11 = (target());
let $tt_v12 = ($tt_v11.v);
{
  const $tt_m = note("assigned scrutinee", shapes[4]);
  switch ($tt_m.kind) {
    case "Point": $tt_v10 = 0; break;
    default: $tt_v10 = 1; break;
  }
}
$tt_v11.v = $tt_v12 += ($tt_v10 === 0 ? note("assigned arm", (box.v = 100, 5)) : 0);
flush("compound assignment", box.v);
let $tt_v13: string;
{
  let $tt_m_1; {
    const $tt_m = note("inner scrutinee", shapes[3]);
    switch ($tt_m.kind) {
      case "Rect": {
        const { w, h } = $tt_m;
        $tt_m_1 = note("inner arm", w < h);
        break;
      }
      default: {
        $tt_m_1 = false;
        break;
      }
    }
  }
  switch ($tt_m_1) {
    case true: {
      $tt_v13 = note("outer true", "tall");
      break;
    }
    case false: {
      $tt_v13 = note("outer false", "flat");
      break;
    }
    default: {
      throw new Error("tt match: unexpected literal " + $tt_show($tt_m_1));
    }
  }
}
const nested = $tt_v13;
flush("nested", nested);
let $tt_v14: number;
const $tt_v15 = (note("before", "<"));
{
  const $tt_m = note("template scrutinee", shapes[0]);
  switch ($tt_m.kind) {
    case "Circle": {
      const { r } = $tt_m;
      $tt_v14 = note("template arm", r);
      break;
    }
    default: {
      $tt_v14 = 0;
      break;
    }
  }
}
const text = `${$tt_v15}${$tt_v14}${note("after", ">")}`;
flush("template", text);
export {};
