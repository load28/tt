//// [variantExportPrefixOnBothDeclarations.tt] ////
export variant Shape { Circle(radius: number), Point }


//// [variantExportPrefixOnBothDeclarations.ts]
export type Shape =
  | { kind: "Circle"; radius: number }
  | { kind: "Point" };
export const Shape = {
  Circle: (radius: number): Shape => ({ kind: "Circle", radius }),
  Point: { kind: "Point" } as const,
};
\ No newline at end of file
