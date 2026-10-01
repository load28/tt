//// [aCaseNamedLikeThePrototypeSetterIsAnOwnConstructorProperty1.tt] ////
variant V { __proto__(x: number), B }
variant U { __proto__, C }


//// [aCaseNamedLikeThePrototypeSetterIsAnOwnConstructorProperty1.ts]
type V =
  | { kind: "__proto__"; x: number }
  | { kind: "B" };
const V = {
  ["__proto__"]: (x: number): V => ({ kind: "__proto__", x }),
  B: { kind: "B" } as const,
};
type U =
  | { kind: "__proto__" }
  | { kind: "C" };
const U = {
  ["__proto__"]: { kind: "__proto__" } as const,
  C: { kind: "C" } as const,
};
