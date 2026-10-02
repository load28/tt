//// [ifLetElseContinuationComments.tt] ////
// Continuation trivia belongs to the if-let site and survives every link.
export variant V { A(value: number), B(value: number), C }
export function read(v: V): number {
  if let A(value) = v {
    return value;
  } /* before else */ else /* after else */ if /* nested head */ let B(value) = v {
    return value * 2;
  } // before final else
  else // after final else
  {
    return 0;
  }
}
console.log([V.A(2), V.B(3), V.C].map(read).join(","));


//// [ifLetElseContinuationComments.ts]
// Continuation trivia belongs to the if-let site and survives every link.
export type V =
  | { kind: "A"; value: number }
  | { kind: "B"; value: number }
  | { kind: "C" };
export const V = {
  A: (value: number): V => ({ kind: "A", value }),
  B: (value: number): V => ({ kind: "B", value }),
  C: { kind: "C" } as const,
};
export function read(v: V): number {
  {
    const $tt_t0 = v;
    if ($tt_t0.kind === "A") {
      const { value } = $tt_t0;
      return value;
    }
    /* before else */
    /* after else */
    else {
      const $tt_t1 = v;
      if ($tt_t1.kind === "B") {
        const { value } = $tt_t1;
        return value * 2;
      }
      /* nested head */
      // before final else
      // after final else
      else {
        return 0;
      }
    }
  }
}
console.log([V.A(2), V.B(3), V.C].map(read).join(","));
