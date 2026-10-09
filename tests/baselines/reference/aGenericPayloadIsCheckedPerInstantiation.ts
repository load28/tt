//// [aGenericPayloadIsCheckedPerInstantiation.tt] ////
variant Shape { Circle(radius: number), Point }
variant Color { Red, Green }
variant Box<T> { Full(item: T), Empty }
export function f(c: Box<Color>) {
  return match (c) { Full(item: Red()) => 1, Full(item: Green()) => 2, Empty => 3 };
}
export function g(b: Box<Shape>) {
  return match (b) { Full(item: Circle(radius)) => radius, Full(item: Point()) => 0, Empty => 1 };
}
export function h(b: Box<Shape>) {
  return match (b) { Full(item: Circle(radius)) => radius, Empty => 1 };
}

