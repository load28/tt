//// [aFieldTypoFixKeepsAShorthandBinding.tt] ////
variant Shape { Circle(radius: number), Rect(w: number), Point }
declare const s: Shape;
export const a = match (s) { Circle(raduis) => raduis, Rect(w) => w, Point => 0 };
export const b = match (s) { Circle(raduis: r) => r, Rect(w) => w, Point => 0 };

