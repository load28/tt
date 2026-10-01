//// [variantSupportsGenerics.tt] ////
variant Pair<T> { First, Second }


//// [variantSupportsGenerics.ts]
type Pair<T> =
  | { kind: "First" }
  | { kind: "Second" };
const Pair = {
  First: { kind: "First" } as const,
  Second: { kind: "Second" } as const,
};
