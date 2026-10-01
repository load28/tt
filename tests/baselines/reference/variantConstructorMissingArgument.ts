//// [variantConstructorMissingArgument.tt] ////
// A missing argument of a variant constructor: TypeScript's related place is
// the constructor's parameter, which is the field the user declared.
export variant Shape { Circle(radius: number), Rect(width: number, height?: string) }
export const c = Shape.Circle();
export const r = Shape.Rect();


//// [variantConstructorMissingArgument.ts]
// A missing argument of a variant constructor: TypeScript's related place is
// the constructor's parameter, which is the field the user declared.
export type Shape =
  | { kind: "Circle"; radius: number }
  | { kind: "Rect"; width: number; height?: string };
export const Shape = {
  Circle: (radius: number): Shape => ({ kind: "Circle", radius }),
  Rect: (width: number, height?: string): Shape => ({ kind: "Rect", width, ...(height === undefined ? {} : { height }) }),
};
export const c = Shape.Circle();
export const r = Shape.Rect();
