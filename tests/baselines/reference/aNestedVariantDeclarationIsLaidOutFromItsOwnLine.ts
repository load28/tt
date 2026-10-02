//// [aNestedVariantDeclarationIsLaidOutFromItsOwnLine.tt] ////
function make() {
  variant Inner { A(x: number), B }
  return Inner.B;
}


//// [aNestedVariantDeclarationIsLaidOutFromItsOwnLine.ts]
function make() {
  type Inner =
    | { kind: "A"; x: number }
    | { kind: "B" };
  const Inner = {
    A: (x: number): Inner => ({ kind: "A", x }),
    B: { kind: "B" } as const,
  };
  return Inner.B;
}
