//// [aNestedDuplicateArmDrawsNoCheckerConsequence.tt] ////
import type { TOption } from "@tt/std";
variant Shape { Circle(radius: number), Point }
export function f(t: TOption<Shape>) {
  return match (t) { Some(value) => 0, None => 1, Some(value: Point()) => 9 };
}

