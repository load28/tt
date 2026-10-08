//// [anOrPatternAlternativeIsNotSuggestedForItsSibling.tt] ////
variant Sh { Circle(r: number), Circles(r: number), Rect }
declare const s: Sh;
export const b = match (s) { Circle(r) | Circel(r) => r, Rect => 0 };

