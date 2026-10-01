//// [variantComplexFieldTypes.tt] ////

variant Node {
  Leaf(entries: Map<string, number[]>),
  Branch(children: Array<string>, meta: { tag: string, depth: number }),
}


//// [variantComplexFieldTypes.ts]

type Node =
  | { kind: "Leaf"; entries: Map<string, number[]> }
  | { kind: "Branch"; children: Array<string>; meta: { tag: string, depth: number } };
const Node = {
  Leaf: (entries: Map<string, number[]>): Node => ({ kind: "Leaf", entries }),
  Branch: (children: Array<string>, meta: { tag: string, depth: number }): Node => ({ kind: "Branch", children, meta }),
};
