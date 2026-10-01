//// [nonExhaustiveMatchIsAnTtcErrorWithPosition.tt] ////
variant Shape { Circle(radius: number), Rect(w: number, h: number), Point }
const f = (s: Shape) => match (s) {
  Circle(radius) => radius,
  Point => 0,
};

