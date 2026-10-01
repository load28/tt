//// [anOptionalVariantFieldIsSetOnlyWhenItsArgumentIs.tt] ////
variant V { C(req: string, opt?: number), D }


//// [anOptionalVariantFieldIsSetOnlyWhenItsArgumentIs.ts]
type V =
  | { kind: "C"; req: string; opt?: number }
  | { kind: "D" };
const V = {
  C: (req: string, opt?: number): V => ({ kind: "C", req, ...(opt === undefined ? {} : { opt }) }),
  D: { kind: "D" } as const,
};
