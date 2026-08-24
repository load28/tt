export function $tt_ap<A, B>(v: A, f: (v: A) => B): B {
  return f(v);
}

export function $tt_fl<A extends unknown[], B, C>(
  f: (...a: A) => B,
  g: (b: B) => C,
): (...a: A) => C {
  return (...a: A) => g(f(...a));
}
