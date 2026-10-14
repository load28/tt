//// [aMatchArgumentOfAnOptionalCallAfterAnEarlierConditionalKeepsItsPlacementError.tt] ////
variant S { A }
declare const s: S;
declare const g: { m?: (a: number, b: number | undefined) => void };
declare const h: ((x: number) => number) | undefined;
declare const x: boolean;

x && match (s) { A => 0 }, g.m?.(match (s) { A => 1 }, h?.(match (s) { A => 2 }));

export {};

