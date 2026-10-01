//// [untypedTryMethodsSurviveNextToTtConstructs.tt] ////
variant V { A }
interface X { try(x); }


//// [untypedTryMethodsSurviveNextToTtConstructs.ts]
type V =
  { kind: "A" };
const V = {
  A: { kind: "A" } as const,
};
interface X { try(x); }
