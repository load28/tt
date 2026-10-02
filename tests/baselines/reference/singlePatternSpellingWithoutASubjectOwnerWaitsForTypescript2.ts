//// [singlePatternSpellingWithoutASubjectOwnerWaitsForTypescript2.tt] ////
variant Shape { Circle(radius: number), Empty }
if let Circel(radius) = s { log(radius); }


//// [singlePatternSpellingWithoutASubjectOwnerWaitsForTypescript2.ts]
type Shape =
  | { kind: "Circle"; radius: number }
  | { kind: "Empty" };
const Shape = {
  Circle: (radius: number): Shape => ({ kind: "Circle", radius }),
  Empty: { kind: "Empty" } as const,
};
{
  const $tt_t0 = s;
  if ($tt_t0.kind === "Circel") {
    const { radius } = $tt_t0;
    log(radius);
  }
}
