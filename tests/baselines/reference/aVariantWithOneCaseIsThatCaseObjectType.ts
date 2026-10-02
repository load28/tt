//// [aVariantWithOneCaseIsThatCaseObjectType.tt] ////
export variant Pub { Item(n: number) }
variant Lone { Only }
variant Noted {
  Cell(
    v: number // value
  )
}


//// [aVariantWithOneCaseIsThatCaseObjectType.ts]
export type Pub =
  { kind: "Item"; n: number };
export const Pub = {
  Item: (n: number): Pub => ({ kind: "Item", n }),
};
type Lone =
  { kind: "Only" };
const Lone = {
  Only: { kind: "Only" } as const,
};
type Noted =
  {
    kind: "Cell";
    v: number; // value
  };
const Noted = {
  Cell: (v: number): Noted => ({ kind: "Cell", v }),
};
