//// [compileStillReturnsTheFirstErrorInSourceOrder.tt] ////
variant Shape { Circle(r: number), Square(s: number) }
export function f(x: Shape): number {
  return match (x) { Circle(r) => r };
}
export function g(x: Shape): number {
  return match (x) { Square(s) => s };
}

