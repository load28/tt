//// [singlePatternSpellingWithoutASubjectOwnerWaitsForTypescript1.tt] ////
variant Shape { Circle(radius: number), Empty }
function f(): number {
  const Circel(radius) = s else { return 0; };
  return radius;
}


//// [singlePatternSpellingWithoutASubjectOwnerWaitsForTypescript1.ts]
type Shape =
  | { kind: "Circle"; radius: number }
  | { kind: "Empty" };
const Shape = {
  Circle: (radius: number): Shape => ({ kind: "Circle", radius }),
  Empty: { kind: "Empty" } as const,
};
function f(): number {
  const $tt_t0 = s;
  if ($tt_t0.kind !== "Circel") {
    return 0;
  }
  const { radius } = $tt_t0;
  return radius;
}
