//// [a.tt] ////
export const x = 1;
export interface X { n: number }

//// [b.tt] ////
export variant V { A(m: typeof import("./a.tt")), B }
export declare variant W { C(n: import("./a.tt").X) }
export const v: V = V.A({ x: 1 });


//// [a.ts]
export const x = 1;
export interface X { n: number }
\ No newline at end of file

//// [b.ts]
export type V =
  | { kind: "A"; m: typeof import("./a.js") }
  | { kind: "B" };
export const V = {
  A: (m: typeof import("./a.js")): V => ({ kind: "A", m }),
  B: { kind: "B" } as const,
};
export declare type W =
  { kind: "C"; n: import("./a.js").X };
export declare const W: {
  readonly C: (n: import("./a.js").X) => W;
};
export const v: V = V.A({ x: 1 });
