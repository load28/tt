//// [armWithoutBody.tt] ////
// Repro from TASK-605
variant Shape { Circle(radius: number), Rect(width: number) }
declare const s: Shape;
export const a = match (s) { Circle(radius) => radius, Rect(width) if width > 0, _ => 0 };

