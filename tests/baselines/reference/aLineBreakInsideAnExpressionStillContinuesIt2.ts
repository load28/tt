//// [aLineBreakInsideAnExpressionStillContinuesIt2.tt] ////
variant O { Some(value: number), None }
declare let q: number;
declare const o: O;
declare const p: ((x: number) => number) | undefined;
const x = p!
  (if let Some(value) = o { value });

