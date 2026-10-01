//// [valIsCheckedInsideNestedTtConstructs2.tt] ////
variant Shape { Circle(r: number), Point }
val const s = Shape.Circle(1);
const v = match (s) {
  Circle(r) => { s.kind = "Point"; return r; },
  Point => 0,
};

