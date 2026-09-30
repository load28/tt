export enum Color { Red, /*green*/Green = 4 }
export namespace Geometry {
  export const unit = 1;
  export function scale(n: number): number;
  export function scale(n: string): string;
  export function scale(n: number | string) { return n; }
}
export abstract class Base<T extends object> {
  protected abstract read(): T;
  static create() { return 1; }
}
export const pair = { a: 1, b: "x" } satisfies Record<string, unknown>;
export const s = Geometry./*scale*/scale(/*overload*/2);
export const c = Color./*member*/Green;
let later = 1;
later = /*assigned*/later + Geometry.unit;
