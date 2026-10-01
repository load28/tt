//// [aCommentAfterTheLastFieldOrCaseStaysAComment.tt] ////
export variant Shape {
  Rect(
    w: number,
    h: number // height
  ),
  Point // last
}


//// [aCommentAfterTheLastFieldOrCaseStaysAComment.ts]
export type Shape =
  | {
      kind: "Rect";
      w: number;
      h: number; // height
    }
  | { kind: "Point" }; // last
export const Shape = {
  Rect: (w: number, h: number): Shape => ({ kind: "Rect", w, h }),
  Point: { kind: "Point" } as const,
};
